#!/usr/bin/env python3
"""Runs copydetect (filtering off) and writes only numbers: per test file, the highest overlap
fraction against any reference file, and the project it came from.

CLEAN-ROOM: never call get_copied_code_list() or generate_html_report(), never print or store
code snippets, and never print reference file contents. Reference files are named
ref-<project>-NNNN.<ext>.
"""
import json
import os
import re
import sys

from copydetect import CopyDetector

NOISE = 25  # characters; copydetect's default
EXTENSIONS = ["rs", "ts", "hs", "py", "c", "h", "cc", "cpp", "hh", "hpp", "js"]


def main(new_dir, ref_dir, out_file):
    det = CopyDetector(
        test_dirs=[new_dir],
        ref_dirs=[ref_dir],
        extensions=EXTENSIONS,
        noise_t=NOISE,
        guarantee_t=NOISE,
        disable_filtering=True,
        autoopen=False,
        silent=True,
    )
    det.run()
    sim = det.similarity_matrix
    if sim.size == 0:
        raise RuntimeError("no similarity matrix")
    result = {}
    for i, test_file in enumerate(det.test_files):
        scores = sim[i, :, 0]
        j = int(scores.argmax())
        score = max(float(scores[j]), 0.0)
        project = re.match(r"ref-(.+)-\d+\.\w+$", os.path.basename(det.ref_files[j]))
        result[os.path.splitext(os.path.basename(test_file))[0]] = {
            "score": score,
            "project": project.group(1) if project and score > 0 else None,
        }
    with open(out_file, "w") as fh:
        json.dump(result, fh)


if __name__ == "__main__":
    try:
        main(*sys.argv[1:4])
    except Exception as e:  # the message can quote source; print the class only
        print(type(e).__name__, file=sys.stderr)
        sys.exit(1)
