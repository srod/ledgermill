#!/usr/bin/env python3
"""A model of what bean-format does, written from running it (spec/beancount/format.md).

`bean_format(text)` reproduces bean-format 3.2.3 on LF text; `proposed(text)` is the variant ledgermill's
format() follows (same output, except that the column is worked out from the indent the lines are given,
which makes it idempotent). Standard library only.

usage: spec/beancount/format-model.py [N [SEED]]
  Writes N random journals (default 300), runs bean-format on each and reports
    - files where bean_format() differs from bean-format (must be 0),
    - files where proposed() is not idempotent (must be 0),
    - files with one indent where proposed() differs from bean-format (must be 0),
    - how often bean-format itself is not idempotent.
"""
import random
import re
import subprocess
import sys
import tempfile
import unicodedata
from collections import Counter


def ranges(wanted):
    """The code points whose Unicode general category is in `wanted`, as a regex class body."""
    out, start = [], None
    for c in range(0x110001):
        ok = c < 0x110000 and not 0xD800 <= c <= 0xDFFF and unicodedata.category(chr(c)) in wanted
        if ok and start is None:
            start = c
        elif not ok and start is not None:
            out.append(chr(start) if start == c - 1 else f"{chr(start)}-{chr(c - 1)}")
            start = None
    return re.sub(r"([\]\\^])", r"\\\1", "".join(out))


UPPER = ranges({"Lu"})
LETTER_OR_DIGIT = ranges({"Lu", "Ll", "Lt", "Lm", "Lo", "Nd"})
UPPER_OR_DIGIT = ranges({"Lu", "Nd"})

BLANK = r"[ \t]"
# a number: sign, blanks, digits and commas, optional point and digits; or one operation in parentheses
NUM = r"[-+]?\s*[\d,]+(?:\.\d*)?"
NUMBER = rf"(?:{NUM}|\({NUM}\s*[-+*/]\s*{NUM}\))"
# a currency, followed by a word boundary
CURRENCY = r"(?:[A-Z][A-Z0-9'._-]*|/[A-Z0-9'._-]*[A-Z](?:[A-Z0-9'._-]*[A-Z0-9])?)\b"
NAME = rf"[{LETTER_OR_DIGIT}-]*"
ROOT = rf"[{UPPER}]{NAME}"
COMPONENT = rf"[{UPPER_OR_DIGIT}]{NAME}"
ACCOUNT = rf"{ROOT}(?::{COMPONENT})+"

POSTING = re.compile(rf"^({BLANK}+)({ACCOUNT})(\s+)({NUMBER})\s+({CURRENCY}.*)$")
DATED = re.compile(rf'^(\d[^";]*?)\s+({NUMBER})\s+({CURRENCY}.*)$')
ACCOUNT_LINE = re.compile(rf"^({BLANK}+)({ROOT}:{COMPONENT}.*)$")


def classify(lines):
    kinds = []
    for line in lines:
        for kind, pattern in (("posting", POSTING), ("dated", DATED), ("account", ACCOUNT_LINE)):
            m = pattern.match(line)
            if m:
                kinds.append((kind, m))
                break
        else:
            kinds.append((None, None))
    return kinds


def commonest_indent(kinds):
    count = Counter(len(m.group(1)) for kind, m in kinds if kind in ("posting", "account"))
    if not count:
        return 0
    top = max(count.values())
    return max(indent for indent, n in count.items() if n == top)  # a tie goes to the larger indent


def align(text, own_indent_width):
    lines = text.split("\n")
    kinds = classify(lines)
    indent = commonest_indent(kinds)
    prefix_width = number_width = 0
    for kind, m in kinds:
        if kind == "posting":
            used = indent if own_indent_width else len(m.group(1))
            prefix_width = max(prefix_width, used + len(m.group(2)))
            number_width = max(number_width, len(m.group(4)))
        elif kind == "dated":
            prefix_width = max(prefix_width, len(m.group(1)))
            number_width = max(number_width, len(m.group(2)))
    out = []
    for line, (kind, m) in zip(lines, kinds):
        if kind == "posting":
            head = (" " * indent + m.group(2)).ljust(prefix_width)
            out.append(f"{head}  {m.group(4).rjust(number_width)} {m.group(5)}")
        elif kind == "dated":
            out.append(f"{m.group(1).ljust(prefix_width)}  {m.group(2).rjust(number_width)} {m.group(3)}")
        elif kind == "account":
            out.append(" " * indent + m.group(2))
        else:
            out.append(line)
    return "\n".join(out)


def bean_format(text):
    return align(text, own_indent_width=False)


def proposed(text):
    return align(text, own_indent_width=True)


# ---- the check against bean-format ----

ACCOUNTS = ["Assets:Bank", "Expenses:Food", "Income:Pay-Check", "Assets:Cash:Longaccountname:X", "Assets:A1", "Liabilities:Card:2024"]
NUMBERS = ["1", "12.5", "-5", "+3.00", "1,000.00", "-1,234,567.891", "0.5", "5.", "(1+2)", "(1 + 2)", "- 5", "(2*3.5)"]
CURRENCIES = ["EUR", "USD", "AAA", "VT-1", "USD.A", "/NQ", "X"]
LETTERS = list(' \t"A;:-.,1a/(+*!@#_~$éÉ日٣')


def seed_line(rng, indent):
    r = rng.random()
    ind = indent or rng.choice(["  ", "  ", "   ", "    ", " ", "\t"])
    acct, num, cur = rng.choice(ACCOUNTS), rng.choice(NUMBERS), rng.choice(CURRENCIES)
    if r < 0.45:
        tail = rng.choice(["", "", " ; c", " {1 EUR}", " @ 2 USD", "  ;x", "\t"])
        return ind + acct + rng.choice(["  ", "   ", " ", "\t"]) + num + rng.choice([" ", "  ", "\t"]) + cur + tail
    if r < 0.6:
        return ind + acct + rng.choice(["", " ;c", "  "])
    if r < 0.7:
        return f"{ind}! {acct}  {num} {cur}"
    if r < 0.8:
        return f"2026-01-0{rng.randint(1, 9)} balance {acct}{rng.choice([' ', '  ', chr(9)])}{num} {cur}"
    if r < 0.86:
        return f"2026-01-0{rng.randint(1, 9)} price {cur} {num} {rng.choice(CURRENCIES)}"
    if r < 0.9:
        return f"2026-01-0{rng.randint(1, 9)} balance {acct} {num} ~ {rng.choice(NUMBERS)} {cur}"
    if r < 0.93:
        return '2026-01-02 * "pay" "x"'
    if r < 0.96:
        return rng.choice(["", ind + 'k: "v"', ind + "; comment"])
    return rng.choice(['2026-01-01 open Assets:Bank EUR', '2026-01-01 custom "b" 5 EUR', 'option "a" "b"'])


def mutate(rng, line):
    if rng.random() < 0.3:
        return line
    for _ in range(rng.randint(1, 3)):
        i = rng.randint(0, len(line))
        r = rng.random()
        if r < 0.5 or not line:
            line = line[:i] + rng.choice(LETTERS) + line[i:]
        elif r < 0.8:
            line = line[:i] + line[i + 1:]
        else:
            i = min(i, len(line) - 1)
            line = line[:i] + rng.choice(LETTERS) + line[i + 1:]
    return line


def run_bean_format(text):
    with tempfile.NamedTemporaryFile("w", suffix=".beancount", encoding="utf-8", newline="") as f:
        f.write(text)
        f.flush()
        r = subprocess.run(["bean-format", f.name], capture_output=True)
    return r.stdout.decode("utf-8") if r.returncode == 0 else None


def main():
    count = int(sys.argv[1]) if len(sys.argv) > 1 else 300
    rng = random.Random(int(sys.argv[2]) if len(sys.argv) > 2 else 1)
    wrong = not_idempotent = uniform = uniform_wrong = bf_not_idempotent = skipped = 0
    for n in range(count):
        indent = "  " if n % 2 else None  # every other file has one indent
        lines = [mutate(rng, seed_line(rng, indent)) for _ in range(rng.randint(3, 25))]
        text = "\n".join(lines) + "\n"
        reference = run_bean_format(text)
        if reference is None:  # bean-format stops with an error (a non-ASCII blank in an amount line)
            skipped += 1
            continue
        if bean_format(text) != reference:
            wrong += 1
            print("bean_format() differs on:", repr(text)[:300])
        mine = proposed(text)
        if proposed(mine) != mine:
            not_idempotent += 1
        if run_bean_format(reference) != reference:
            bf_not_idempotent += 1
        kinds = classify(text.split("\n"))
        if len({len(m.group(1)) for k, m in kinds if k in ("posting", "account")}) <= 1:
            uniform += 1
            if mine != reference:
                uniform_wrong += 1
                print("proposed() differs on a file with one indent:", repr(text)[:300])
    done = count - skipped
    print(f"{done} files ({skipped} skipped: bean-format fails)")
    print(f"  bean_format() differs from bean-format: {wrong}")
    print(f"  proposed() not idempotent: {not_idempotent}")
    print(f"  one-indent files: {uniform}, proposed() differs from bean-format on {uniform_wrong}")
    print(f"  bean-format not idempotent on {bf_not_idempotent}")


if __name__ == "__main__":
    main()
