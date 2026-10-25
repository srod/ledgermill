# Similarity check

Before each release, `run.sh` compares our code with pinned copies of the upstream sources, to
catch code an AI model memorised and wrote out. Nobody reads the upstream trees: the script
downloads them, gives every file an opaque name (`ref-0001.rs`), and prints only our own paths
and numbers.

```sh
scripts/similarity/run.sh
```

Exit 0: pass. Exit 1: a `crates/*/src` file scored 0.15 or more, or JPlag could not check one of
our files. Exit 2: the check could not run (missing tool, failed download, checksum mismatch).

The last line is the score to record in the release notes:

```
similarity: max 0.030 (crates/ledgermill/src/parse.rs), 0 fail, 0 warn; copydetect max 0.010
```

With `GITHUB_STEP_SUMMARY` set, a Markdown table is appended there as well.

Files: `run.sh` (entry point), `lock.tsv` (pins), `jplag_prepare.py` (let-else and let-chain
rewrite), `copydetect_scores.py`, `report.py` (reads JPlag's JSON, prints, decides the exit code),
`selftest.sh`.

## What it needs

- Java 25 or newer for JPlag 6.3.0. The script looks at `JAVA_HOME_25_X64`,
  `JAVA_HOME_25_arm64`, `JAVA_HOME`, Homebrew's `openjdk` and then `java` on `PATH`. The
  ubuntu-24.04 runners have JDK 25 installed; locally use `brew install openjdk`.
- `python3` with `venv`, `curl`, `tar`, `git`. copydetect is installed into
  `~/.cache/ledgermill-similarity/venv` on first use.
- Network on the first run. Downloads are cached in `~/.cache/ledgermill-similarity`
  (`LEDGERMILL_SIMILARITY_CACHE` overrides it) and checked against `lock.tsv` every run, so a CI
  job can cache that directory. Nothing under the cache is inside the repository.

## The two tools

**JPlag is the gate.** It turns each Rust file into tokens (identifiers and literals are ignored,
so renamed variables still match) and looks for runs of at least 25 tokens, about six lines,
shared with a rustledger 0.24.0 file. Our `crates/*/src/**/*.rs` files are the new submissions,
every rustledger `.rs` file is an old one, one file per submission. The score of one of our files
is the largest, over rustledger files, of matched tokens divided by our file's tokens.

- 0.15 or more in a `src/` file: **FAIL**.
- 0.05 or more: **WARN**. Look at the line ranges and think about where the code came from.
- Files under `crates/*/tests` (and anything else outside `src/`) are reported but never gate.
- A file JPlag cannot parse would silently score zero, so it fails the run. Files with under 25
  tokens (a test full of `json!` bodies, say) cannot contain a 25-token match; they are listed in
  the output.

**copydetect is informational.** It cuts the text into 25-character windows, with code filtering
off, and reports how much of each of our files (`.rs` and `harness/*.ts`) appears in rustledger,
hledger-lib, hledger, Beancount 3.2.3 and Ledger 3.4.1, whatever the language. It never fails
the run.

Both only ever report our line numbers and numbers. If you need to act on a result, go to the
lines named in our file and rewrite them from the spec notes, without looking upstream.

## What it cannot catch

- Logic rewritten in another language (Haskell or Python logic turned into Rust), or with
  renamed identifiers and reordered statements. JPlag ignores names, copydetect does not.
- Copies shorter than about six lines.
- Code inside macro bodies (`json!`, `macro_rules!`): JPlag's Rust grammar barely tokenises it.
- Anything the Rust grammar does not know. JPlag 6.3.0 cannot parse `let ... else` or let-chains
  (`if a && let Some(x) = y`); it recovers with a run of junk tokens that looks the same in
  unrelated code, and a first run scored two unrelated functions of ours at 0.59. `jplag_prepare.py`
  rewrites both forms in the copies it hands to JPlag, for ours and upstream alike. Other gaps
  show up as "grammar-recovery messages" in the first output line: 0 for our files at the time of
  writing, so the count is upstream's. If a new Rust feature makes it rise for ours, add a rewrite
  there before trusting the scores.
- Anything not in the pinned versions, and files in `crates/` that are not tracked or new.
- Memorised code that is only similar in shape, not in tokens.

## Reading the numbers

JPlag ignores names and literals, so lists and tables of the same shape match: a `mod a;` list, a
table of `("january", 1)` pairs, a run of enum variants. The first run against the pinned
rustledger (2026-10-25: max 0.102, 0 fail, 8 warn) had one run of 26 to 29 tokens, the minimum, in
each WARN file, except the month table in `period.rs` (118 tokens). Tests score higher because
`json!` and `assert_eq!` bodies leave few tokens; that is why they do not gate.

The same run against a control, eight unrelated MIT/Apache crates (regex, serde_json, chrono,
rust_decimal, winnow, clap_builder, itertools, anyhow), gave max 0.252, 1 fail, 2 warn. The fail was
a single 27-token run in `problem.rs`, a file of 107 tokens. So a small file can reach 0.15 with one
minimum-length run. Look at the lines before acting: a short, generic run is not a copy. The
thresholds are kept as decided; if this makes the gate flaky, the fix to weigh is also requiring
a minimum number of matched tokens (say 60) for a FAIL, not a higher percentage.

`SIMILARITY_MIN_TOKENS`, `SIMILARITY_FAIL` and `SIMILARITY_WARN` override the 25 / 0.15 / 0.05
for calibration runs. A release uses the defaults.

copydetect at 25 characters with filtering off is noisier still: files of short `use` and `mod`
lines reach 0.2 to 0.3 against any Rust. Treat it as a pointer to look at, never as a verdict.

A pass is evidence, not proof. It sits next to the exposure register and the spec notes.

## Clean-room rules for this directory

- Never open, list or print anything from the downloaded archives, the extracted trees or a
  `.jplag` report's `files/` folder. `report.py` refuses to read anything outside the JSON
  metadata.
- The JPlag and copydetect logs can quote source (parse errors do), so `run.sh` never prints
  them. Do not add a debugging `cat` of either.
- copydetect's API can return code snippets. Use only the numeric scores, as
  `copydetect_scores.py` does.
- The temp dir is deleted on exit (`trap`), including after a failure.
- If a tool ever shows upstream source or file names, add a row to `EXPOSURE.md`.

## Pins

`lock.tsv` lists every input with its SHA-256. A mismatch aborts the run; the file is not used.
Hackage and crates.io checksums come from their metadata; GitHub archives and the JPlag jar are
pinned to the checksum of their first download. GitHub could in theory change how it builds tag
archives; if the hash changes for no other reason, check the tag and re-pin it in a commit of its
own. To move to a new upstream version, change the URL and checksum in the same commit as
the release that uses it. copydetect is pinned by version (`COPYDETECT_VERSION` in `run.sh`).

JPlag is GPL-3.0 and copydetect is MIT. We run them as separate programs and do not link them or
ship them.

## Testing the script

`selftest.sh` runs `run.sh` against a stand-in made from our own files (no upstream download). It
expects a copied file to fail and unrelated code to pass.
