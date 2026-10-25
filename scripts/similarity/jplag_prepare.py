#!/usr/bin/env python3
"""Rewrites two newer Rust forms that JPlag 6.3.0's grammar cannot parse, in place, for the files
in the given directories: let-else (`let PAT = EXPR else { .. };`) and let-chains
(`if a && let PAT = EXPR && b`).

The grammar recovers from each with a run of garbage tokens whose shape is the same in unrelated
code: two functions with nothing in common matched for hundreds of tokens. The rewrites turn
let-else's `else` into `;` (a let and a block) and drop the `let` of a chain (leaving an
assignment expression). They are applied to both our files and the upstream ones, so the two stay
comparable, and only to the copies the script makes.

CLEAN-ROOM: reads and rewrites files without printing any of them or their names.
"""
import os
import re
import sys

# Alternatives in order: comments, raw strings, strings, char literals, words, spaces, anything.
TOKEN = re.compile(
    r"""//[^\n]*|/\*.*?\*/|b?r(#*)".*?"\1|b?"(?:\\.|[^"\\])*"|'(?:\\[^']*|[^\\'])'|[A-Za-z_]\w*|\s+|.""",
    re.S,
)


def convert(src):
    out, last = [], ""  # last: the final characters of the code so far, "a" standing for a word
    for m in TOKEN.finditer(src):
        t = m.group(0)
        if t.startswith("//") or t.startswith("/*") or t.isspace():
            out.append(t)
            continue
        # An `else` that does not follow a `}` belongs to a let-else: Rust forbids a `}`
        # right before the `else` of a let.
        if t == "else" and not last.endswith("}"):
            out.append(";")
        elif t == "let" and last == "&&":
            out.append("")
        else:
            out.append(t)
        last = (last + ("a" if (t[0].isalpha() or t[0] == "_") else t[-1]))[-2:]
    return "".join(out)


def main(dirs):
    for d in dirs:
        for name in os.listdir(d):
            if name.endswith(".rs"):
                path = os.path.join(d, name)
                with open(path, encoding="utf-8", errors="replace") as fh:
                    src = fh.read()
                with open(path, "w", encoding="utf-8") as fh:
                    fh.write(convert(src))


if __name__ == "__main__":
    try:
        main(sys.argv[1:])
    except Exception as e:  # the message can quote source; print the class only
        print(type(e).__name__, file=sys.stderr)
        sys.exit(1)
