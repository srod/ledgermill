# Legal risk note

This is the maintainer's own assessment, not legal advice. No lawyer has reviewed it.

## Why we think a clean-room engine is allowed

- In the EU, what a program does, its programming language and its data-file formats are not
  protected by copyright (Court of Justice, *SAS Institute v World Programming*, C-406/10, 2012).
  Anyone who lawfully uses a program may observe, study and test it to learn the ideas behind it
  (Directive 2009/24/EC, art. 5(3)). That is how ledgermill learns behaviour.
- What is protected is the expression: source code, and the wording of manuals. We never read
  the former and never copy the latter.
- In France (CPI L122-6-1) the same rules apply, and there is no fair use to rely on, so we don't.
  In the US, observing behaviour and reimplementing it is also supported (*Sega v. Accolade*,
  *Lotus v. Borland*, *Google v. Oracle*), so the EU-safe method is US-safe too.

## What remains open

1. **File formats.** The EU judgment leaves open whether a file format could be protected some
   other way. A line-oriented text format seems a poor candidate, but no court has said so for
   hledger or Beancount files.
2. **Report layout.** We do not reproduce report layouts, and only match field names and values.
   Whether matching a layout would be copying has not been decided by any court.
3. **AI memorisation.** AI models were trained on public code, likely including GPL ledger
   tools. No court or policy says what happens when a model reproduces memorised code. Our
   guard is the similarity check before each release.

We judge the risk low because we copy no expression, keep evidence of where every rule came
from (spec notes with our own journals), and record exposure.

## If someone complains

1. We take the complaint seriously and reply.
2. We identify the code concerned and check the exposure register and spec notes for that area.
3. If there is any doubt, we delete that code and rewrite it from the spec notes in a fresh
   session with no exposure, then release.

A paid legal review will be reconsidered if the project gets funding or sponsors, if a company
comes to depend on it, or if a complaint is made.
