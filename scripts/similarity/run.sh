#!/usr/bin/env bash
# Compares our code with the pinned upstream sources. See README.md.
#
# CLEAN-ROOM: nobody reads upstream files, including this script's output. Upstream files get
# opaque names, tool logs are never printed (they can quote source), and the temp dir is
# deleted on exit. Do not add `ls`, `cat`, `set -x` or log output for the upstream trees.
#
# Exit: 0 pass, 1 gate failed, 2 the check could not run.
set -euo pipefail

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
CACHE=${LEDGERMILL_SIMILARITY_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}/ledgermill-similarity}
COPYDETECT_VERSION=0.5.0
# The defaults are the owner's decision; the overrides are for calibration runs only.
MIN_TOKENS=${SIMILARITY_MIN_TOKENS:-25}
FAIL_AT=${SIMILARITY_FAIL:-0.15}
WARN_AT=${SIMILARITY_WARN:-0.05}
JPLAG_REF=rustledger
# Only for scripts/similarity/selftest.sh: a directory of our own files standing in for upstream.
REF_TREE=${SIMILARITY_REF_TREE:-}
# Source extensions of the upstream projects that copydetect looks at.
EXTS=(rs hs py c h cc cpp hh hpp js ts)

die() { echo "similarity: $*" >&2; exit 2; }

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
trap 'exit 130' INT TERM

sha256() { if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi; }

archive_path() { # slug kind
  if [ "$2" = jar ]; then echo "$CACHE/$1.jar"; else echo "$CACHE/$1.tar.gz"; fi
}

fetch() { # slug kind sha256 url
  local f; f=$(archive_path "$1" "$2")
  if [ -f "$f" ] && [ "$(sha256 "$f")" = "$3" ]; then return; fi
  rm -f "$f" "$f.part"
  echo "similarity: downloading $1" >&2
  curl -fsSL --retry 3 -o "$f.part" "$4" || die "download of $1 failed"
  if [ "$(sha256 "$f.part")" != "$3" ]; then
    rm -f "$f.part"
    die "checksum mismatch for $1 (lock file says $3); the archive was not used"
  fi
  mv "$f.part" "$f"
}

java_major() { "$1" -version 2>&1 | sed -n '1s/[^"]*"\([0-9]*\).*/\1/p'; }

find_java() { # JPlag 6.3.0 needs Java 25 or newer
  local c m
  for c in "${JAVA_HOME_25_X64:+$JAVA_HOME_25_X64/bin/java}" "${JAVA_HOME_25_arm64:+$JAVA_HOME_25_arm64/bin/java}" \
    "${JAVA_HOME:+$JAVA_HOME/bin/java}" /opt/homebrew/opt/openjdk/bin/java /usr/local/opt/openjdk/bin/java \
    "$(command -v java || true)"; do
    if [ -n "$c" ] && [ -x "$c" ]; then
      m=$(java_major "$c")
      if [ "${m:-0}" -ge 25 ]; then echo "$c"; return; fi
    fi
  done
  return 1
}

# Copies every source file under $2 to a flat opaque name; sets FLAT_N (a count, never names).
# Not run in a subshell: J and FLAT_N must survive.
J=0
FLAT_N=0
flatten() { # proj tree
  local proj=$1 tree=$2 i=0 f ext
  local args=() e
  for e in "${EXTS[@]}"; do args+=(-o -name "*.$e"); done
  while IFS= read -r -d '' f; do
    ext=${f##*.}
    i=$((i + 1))
    cp "$f" "$TMP/cd/ref/ref-$proj-$(printf %04d "$i").$ext" 2>/dev/null || die "could not copy a $proj file"
    if [ "$proj" = "$JPLAG_REF" ] && [ "$ext" = rs ]; then
      J=$((J + 1))
      cp "$f" "$TMP/old/ref-$(printf %04d "$J").rs" 2>/dev/null || die "could not copy a $proj file"
    fi
  done < <(find "$tree" -type f \( -false "${args[@]}" \) -print0 2>/dev/null)
  FLAT_N=$i
}

JAVA=$(find_java) || die "needs Java 25 or newer (set JAVA_HOME, or on macOS: brew install openjdk)"
command -v python3 >/dev/null || die "needs python3"
command -v curl >/dev/null || die "needs curl"
mkdir -p "$CACHE" "$TMP/new" "$TMP/old" "$TMP/cd/new" "$TMP/cd/ref" "$TMP/up"

# Fetch and verify everything first, so a bad pin aborts before any work.
JAR=
while IFS=$'\t' read -r slug kind _licence sha url; do
  case $slug in '' | '#'*) continue ;; esac
  if [ "$kind" = jar ]; then JAR=$(archive_path "$slug" "$kind"); elif [ -n "$REF_TREE" ]; then continue; fi
  fetch "$slug" "$kind" "$sha" "$url"
done <"$HERE/lock.tsv"
[ -n "$JAR" ] || die "lock.tsv has no jar"

# Our files, under opaque ids f-NNNN; ours.tsv maps id -> kind, size, path.
cd "$ROOT"
n=0
: >"$TMP/ours.tsv"
while IFS= read -r p; do
  [ -f "$p" ] || continue
  n=$((n + 1))
  id=$(printf 'f-%04d' "$n")
  ext=${p##*.}
  case $p in
    harness/*) kind=harness ;;
    crates/*/src/*) kind=src ;;
    crates/*/tests/*) kind='test' ;;
    *) kind=other ;;
  esac
  [ "$ext" = rs ] && cp "$p" "$TMP/new/$id.rs"
  cp "$p" "$TMP/cd/new/$id.$ext"
  printf '%s\t%s\t%s\t%s\n' "$id" "$kind" "$(wc -c <"$p" | tr -d ' ')" "$p" >>"$TMP/ours.tsv"
done < <(git ls-files --cached --others --exclude-standard -- 'crates/*.rs' 'harness/*.ts' | LC_ALL=C sort -u)
[ "$n" -gt 0 ] || die "found none of our files; run from a ledgermill checkout"

# Upstream trees -> opaque flat copies.
refs=0
if [ -n "$REF_TREE" ]; then
  JPLAG_REF=fixture
  flatten fixture "$REF_TREE"
  refs=$FLAT_N
else
  while IFS=$'\t' read -r slug kind _licence _sha _url; do
    case $slug in '' | '#'*) continue ;; esac
    [ "$kind" = tar ] || continue
    mkdir "$TMP/up/$slug"
    tar -xzf "$(archive_path "$slug" "$kind")" -C "$TMP/up/$slug" 2>/dev/null || die "could not extract $slug"
    flatten "${slug%%-[0-9]*}" "$TMP/up/$slug"
    refs=$((refs + FLAT_N))
    rm -rf "$TMP/up/$slug"
  done <"$HERE/lock.tsv"
fi
[ "$J" -gt 0 ] || die "no Rust files in the $JPLAG_REF tree"

# JPlag's Rust grammar chokes on let-else; see jplag_prepare.py.
python3 "$HERE/jplag_prepare.py" "$TMP/new" "$TMP/old" || die "could not prepare the Rust files for JPlag"

# JPlag: the gate. Its log can quote source, so it is never printed.
"$JAVA" -jar "$JAR" --mode RUN --language rust --min-tokens "$MIN_TOKENS" \
  --new "$TMP/new" --old "$TMP/old" --shown-comparisons=-1 --cluster-skip \
  --result-file "$TMP/jplag" --overwrite >"$TMP/jplag.log" 2>&1 ||
  die "JPlag failed (exit $?); its log is withheld because it can quote source"
recovered=$(tr '\r' '\n' <"$TMP/jplag.log" | grep -c '^line [0-9]*:[0-9]*' || true)

# copydetect: informational, runs in a venv under the cache dir.
VPY="$CACHE/venv/bin/python"
if ! "$VPY" -c "import sys, importlib.metadata as m; sys.exit(m.version('copydetect') != '$COPYDETECT_VERSION')" 2>/dev/null; then
  echo "similarity: installing copydetect $COPYDETECT_VERSION into $CACHE/venv" >&2
  python3 -m venv "$CACHE/venv" && "$VPY" -m pip install -q "copydetect==$COPYDETECT_VERSION" ||
    die "could not install copydetect"
fi
"$VPY" "$HERE/copydetect_scores.py" "$TMP/cd/new" "$TMP/cd/ref" "$TMP/copydetect.json" \
  >"$TMP/copydetect.log" 2>&1 ||
  die "copydetect failed; its log is withheld because it can quote source"

python3 "$HERE/report.py" --jplag "$TMP/jplag.jplag" --ours "$TMP/ours.tsv" --copydetect "$TMP/copydetect.json" \
  --fail "$FAIL_AT" --warn "$WARN_AT" \
  --context "$J $JPLAG_REF Rust files (JPlag, $MIN_TOKENS tokens, fail at $FAIL_AT, warn at $WARN_AT, $recovered grammar-recovery messages), $refs files in all archives (copydetect $COPYDETECT_VERSION)"
