#!/usr/bin/env bash
# Checks run.sh and report.py on a stand-in for upstream built from our own files (no upstream
# archive is downloaded). A copy of one of our files must fail the gate; unrelated code must pass.
set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1
HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
FIX=$(mktemp -d)
trap 'rm -rf "$FIX"' EXIT

python3 - "$HERE" <<'PY'
import sys
sys.path.insert(0, sys.argv[1])
from jplag_prepare import convert
cases = {
    "let Some(x) = a.b() else { return; };": "let Some(x) = a.b() ; { return; };",
    "if a { 1 } else { 2 }": "if a { 1 } else { 2 }",
    "if a && let Some(x) = f() && b { 1 }": "if a &&  Some(x) = f() && b { 1 }",
    "let x = a && b;": "let x = a && b;",
    "if a {\n} // c\nelse if b {} else {}": "if a {\n} // c\nelse if b {} else {}",
    'let s = "else"; // else\nlet T { a } = t else { panic!() };': 'let s = "else"; // else\nlet T { a } = t ; { panic!() };',
    "let c = 'e'; fn f<'a>() {} let x = y else { z };": "let c = 'e'; fn f<'a>() {} let x = y ; { z };",
}
for src, want in cases.items():
    assert convert(src) == want, src
PY

mkdir "$FIX/copy" "$FIX/clean"
src=$(cd "$ROOT" && git ls-files 'crates/*/src/*.rs' | LC_ALL=C xargs wc -l | grep -v total | sort -rn | sed -n 1p | awk '{print $2}')
cp "$ROOT/$src" "$FIX/copy/renamed.rs"
printf 'fn add(a: u32, b: u32) -> u32 {\n    a + b\n}\n' >"$FIX/clean/tiny.rs"

rc=0
SIMILARITY_REF_TREE="$FIX/copy" "$HERE/run.sh" >"$FIX/copy.out" || rc=$?
[ "$rc" = 1 ] || { echo "selftest: a copied file must exit 1, got $rc"; exit 1; }
tail -n 1 "$FIX/copy.out" | grep -q '^similarity: max 1\.000 (crates/.*), [1-9][0-9]* fail' || { echo "selftest: unexpected summary"; tail -n 1 "$FIX/copy.out"; exit 1; }

SIMILARITY_REF_TREE="$FIX/clean" "$HERE/run.sh" >"$FIX/clean.out" || { echo "selftest: unrelated code must exit 0"; exit 1; }
tail -n 1 "$FIX/clean.out" | grep -q '^similarity: max 0\.000 (none), 0 fail' || { echo "selftest: unexpected summary"; tail -n 1 "$FIX/clean.out"; exit 1; }
echo "selftest: ok"
