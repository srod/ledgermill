#!/usr/bin/env python3
"""Reads a JPlag 6.3.0 report and copydetect's scores and prints the similarity result.

CLEAN-ROOM: this reads only runInformation.json, submissionMappings.json,
submissionFileIndex.json and comparisons/*.json from the report, never files/ (a copy of every
source file). It prints only our paths and numbers; upstream files are opaque ids and are never
printed.

Exit: 0 pass, 1 gate failed, 2 the report is not what we expect.
"""
import argparse
import json
import os
import sys
import zipfile

FAIL = 0.15  # defaults: JPlag score of a src/ file at or above this fails the release
WARN = 0.05
# JPlag cannot find a match of MIN_TOKENS in a file with fewer tokens, so these are not a hole.
# Such files are listed in the output (a file full of macro bodies lands here too). Any other
# state, such as CANNOT_PARSE, means the file was not checked and fails the run.
HARMLESS = {"TOO_SMALL", "NOTHING_TO_PARSE"}
ALLOWED_MEMBERS = {"runInformation.json", "submissionMappings.json", "submissionFileIndex.json"}


class Report:
    def __init__(self, path):
        self.z = zipfile.ZipFile(path)

    def load(self, name):
        if name not in ALLOWED_MEMBERS and not (name.startswith("comparisons/") and name.endswith(".json")):
            raise ValueError("refusing to read a report member that may hold source")
        return json.loads(self.z.read(name))


def merge(ranges):
    out = []
    for a, b in sorted(ranges):
        if out and a <= out[-1][1] + 1:
            out[-1][1] = max(out[-1][1], b)
        else:
            out.append([a, b])
    return out


def fmt_ranges(ranges, limit=8):
    parts = [f"{a}-{b}" if a != b else str(a) for a, b in merge(ranges)]
    more = f" +{len(parts) - limit} more" if len(parts) > limit else ""
    return ",".join(parts[:limit]) + more


def jplag_scores(report):
    """Returns ({id: (score, matched, total, ranges)}, {id: state}) for our Rust files."""
    mapping = report.load("submissionMappings.json")
    tokens = {
        sid: sum(f["tokenCount"] for f in files.values())
        for sid, files in report.load("submissionFileIndex.json")["fileIndexes"].items()
    }
    ids = mapping["submissionIds"]  # report id -> "new/f-0001.rs" or "old/ref-0001.rs"
    stem = lambda disp: os.path.splitext(os.path.basename(disp))[0]
    is_new = lambda sid: ids[sid].startswith("new/")
    run = report.load("runInformation.json")
    failed = {stem(f["submissionId"]): f["submissionState"] for f in run["failedSubmissions"] if f["submissionId"].startswith("new/")}
    old_failed = sum(1 for f in run["failedSubmissions"] if f["submissionId"].startswith("old/"))

    scores = {}
    for sid, disp in ids.items():
        if not is_new(sid) or stem(disp) in failed:
            continue
        total = tokens.get(sid)
        if not total:
            raise ValueError("a file of ours is in the report without tokens")
        best = (0.0, 0, total, [])
        for other, fname in mapping["submissionIdsToComparisonFileName"].get(sid, {}).items():
            if is_new(other):
                continue
            c = report.load("comparisons/" + fname)
            side = "First" if c["firstSubmissionId"] == sid else "Second"
            matches = c["matches"]
            matched = sum(m["lengthOf" + side] for m in matches)
            if matched / total > best[0]:
                best = (matched / total, matched, total, [(m["startIn" + side]["line"], m["endIn" + side]["line"]) for m in matches])
        scores[stem(disp)] = best
    return scores, failed, old_failed


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--jplag", required=True)
    ap.add_argument("--ours", required=True)
    ap.add_argument("--copydetect", required=True)
    ap.add_argument("--context", default="")
    ap.add_argument("--fail", type=float, default=FAIL)
    ap.add_argument("--warn", type=float, default=WARN)
    args = ap.parse_args()
    fail_at, warn_at = args.fail, args.warn

    ours = {}  # id -> (kind, size, path)
    with open(args.ours) as fh:
        for line in fh:
            i, kind, size, path = line.rstrip("\n").split("\t")
            ours[i] = (kind, int(size), path)
    with open(args.copydetect) as fh:
        cd = json.load(fh)

    try:
        scores, failed, old_failed = jplag_scores(Report(args.jplag))
    except (KeyError, ValueError, OSError, zipfile.BadZipFile) as e:
        print(f"similarity: unexpected JPlag report ({type(e).__name__})", file=sys.stderr)
        return 2

    problems = []  # files whose JPlag result cannot be trusted
    skipped = []
    for i, (kind, size, path) in sorted(ours.items(), key=lambda kv: kv[1][2]):
        if not path.endswith(".rs"):
            continue
        if i in failed:
            if failed[i] in HARMLESS:
                skipped.append(f"{path} ({size} bytes)")
            else:
                problems.append(f"{path}: JPlag state {failed[i]}")
        elif i not in scores:
            problems.append(f"{path}: not in the JPlag report")

    rows = []
    for i, (kind, size, path) in ours.items():
        j = scores.get(i)
        jscore = j[0] if j else None
        c = cd.get(i, {"score": 0.0, "project": None})
        if (jscore or 0) > 0 or c["score"] > 0:
            rows.append((path, kind, j, c))
    rows.sort(key=lambda r: (-((r[2] or (0,))[0]), -r[3]["score"], r[0]))

    def status(kind, jscore):
        if kind != "src" or jscore is None:
            return "info"
        return "FAIL" if jscore >= fail_at else "WARN" if jscore >= warn_at else "ok"

    lines, md = [], ["| file | JPlag | tokens | lines | copydetect |", "|---|---|---|---|---|"]
    for path, kind, j, c in rows:
        cdtxt = f"{c['score']:.3f}" + (f" ({c['project']})" if c["project"] else "")
        if j and j[0] > 0:
            jtxt, ttxt, rtxt = f"{j[0]:.3f}", f"{j[1]}/{j[2]}", fmt_ranges(j[3])
        else:
            jtxt, ttxt, rtxt = ("skipped" if j is None else "0.000"), "-", "-"
        lines.append(f"{status(kind, j[0] if j else None):4}  {path}  jplag {jtxt} ({ttxt} tokens) lines {rtxt}  copydetect {cdtxt}")
        md.append(f"| {status(kind, j[0] if j else None)} `{path}` | {jtxt} | {ttxt} | {rtxt} | {cdtxt} |")

    src = [(r[2][0], r[0]) for r in rows if r[1] == "src" and r[2]]
    top = max(src) if src else (0.0, "none")
    nfail = sum(1 for s, _ in src if s >= fail_at)
    nwarn = sum(1 for s, _ in src if warn_at <= s < fail_at)
    cdmax = max((v["score"] for v in cd.values()), default=0.0)

    print(f"similarity: {len(ours)} files of ours; {args.context}")
    print(f"similarity: {old_failed} upstream files JPlag could not use")
    if skipped:
        print(f"similarity: JPlag skipped {len(skipped)} of our files with under 25 tokens: " + ", ".join(skipped))
    for line in lines:
        print(line)
    for p in problems:
        print(f"FAIL  {p}")
    summary = f"similarity: max {top[0]:.3f} ({top[1]}), {nfail} fail, {nwarn} warn; copydetect max {cdmax:.3f}"
    if problems:
        summary += f"; {len(problems)} file(s) JPlag could not check"
    print(summary)

    step = os.environ.get("GITHUB_STEP_SUMMARY")
    if step:
        with open(step, "a") as fh:
            fh.write("## Similarity check\n\n" + "\n".join(md) + f"\n\n`{summary}`\n\n")
    return 1 if (nfail or problems) else 0


if __name__ == "__main__":
    sys.exit(main())
