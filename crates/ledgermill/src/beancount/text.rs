//! Reading the raw text around the parser: line numbers, and the lines that are refused before
//! the parser sees them (it would read the filesystem for `include`, and a few number
//! expressions would make it panic).

use super::expr::{self, Expressions};
use super::{auto_accounts, implicit_prices};
use std::borrow::Cow;
use std::collections::HashSet;

/// Byte offsets where each line starts.
pub(super) fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect()
}

/// 1-based line of a byte offset.
pub(super) fn line_of(starts: &[usize], offset: usize) -> u32 {
    let index = starts.partition_point(|&s| s <= offset);
    u32::try_from(index).unwrap_or(u32::MAX)
}

pub(super) struct Refusal {
    pub line: u32,
    pub construct: &'static str,
    /// The option name for `option`, the module name for `plugin`.
    pub name: Option<String>,
}

/// Options checked to change nothing transactions or postings show
/// (spec/beancount/options-and-directives.md); `booking_method` is read from the parser.
const HARMLESS_OPTIONS: &[&str] = &[
    "title",
    "operating_currency",
    "render_commas",
    "documents",
    "booking_method",
    "long_string_maxlines",
    "conversion_currency",
    "account_previous_balances",
    "account_previous_earnings",
    "account_previous_conversions",
    "account_current_earnings",
    "account_current_conversions",
    "account_unrealized_gains",
    "account_rounding",
];

/// Longest run of digits (and `,` `.` inside it) read as a number.
const MAX_NUMBER_CHARS: usize = 22;

fn starts_with_keyword(line: &str, keyword: &str) -> bool {
    line.strip_prefix(keyword)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', '\t', '"']))
}

/// The line without its quoted strings and its `;` comment.
fn code_of(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_string = false;
    let mut escaped = false;
    for c in line.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
        } else if c == '"' {
            in_string = true;
            out.push(' ');
        } else if c == ';' {
            break;
        } else {
            out.push(c);
        }
    }
    out
}

/// `{*}` and `, *}` mark a lot merge; the `*` is not an operator.
fn merge_marker_removed(code: &str) -> String {
    let chars: Vec<char> = code.chars().collect();
    let mut out = String::with_capacity(code.len());
    for (i, c) in chars.iter().enumerate() {
        let before_brace = chars
            .get(i + 1..)
            .is_some_and(|rest| rest.iter().find(|c| !c.is_whitespace()) == Some(&'}'));
        if !(*c == '*' && before_brace) {
            out.push(*c);
        }
    }
    out
}

fn longest_number(code: &str) -> usize {
    let mut longest = 0;
    let mut run = 0;
    for c in code.chars() {
        if c.is_ascii_digit() || (run > 0 && (c == ',' || c == '.')) {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    longest
}

/// `2026-01-02`, possibly with more after it (`2026-01-02}`).
fn is_date_like(token: &str) -> bool {
    token.len() >= 10
        && token.bytes().take(10).enumerate().all(|(i, b)| match i {
            4 | 7 => b == b'-',
            _ => b.is_ascii_digit(),
        })
}

/// A number in a piece of a cost: a token that starts with a digit or a dot, not a date.
fn has_number(part: &str) -> bool {
    part.split(|c: char| c == ',' || c.is_whitespace())
        .any(|t| {
            let t = t.trim_start_matches(['+', '-', '(']);
            t.starts_with(|c: char| c.is_ascii_digit() || c == '.') && !is_date_like(t)
        })
}

/// `{# 5 EUR}` and `{2 # EUR}`: a `#` with a number missing on one side. The reference works the
/// missing part out (inferred-cost-and-price.md); the rewrite of `{{ total }}` into `{# total}`
/// would make it look like a written total, so it is refused here, on the text as written.
fn has_partial_cost(code: &str) -> bool {
    let mut rest = code;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            return false;
        };
        if let Some((left, right)) = after[..close].split_once('#')
            && !(has_number(left) && has_number(right))
        {
            return true;
        }
        rest = &after[close + 1..];
    }
    false
}

fn option_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("option")?;
    let start = rest.find('"')? + 1;
    let end = rest.get(start..)?.find('"')? + start;
    rest.get(start..end).map(str::to_string)
}

/// Whether a line can be skipped by `prescan`'s number checks without looking at its code: it
/// has none of the characters that make an expression, a tolerance or a partial cost, and no run of digits
/// long enough to refuse. Strings and comments only ever remove characters from the code, so
/// what holds for the whole line holds for the code too (the converse need not hold).
fn plainly_fine(raw: &str) -> bool {
    let mut run = 0usize;
    let mut after_sign = false;
    for b in raw.bytes() {
        if matches!(b, b'(' | b')' | b'/' | b'*' | b'#' | b'~') {
            return false;
        }
        // Two signs with only blanks between: `--4`, looked at closer.
        if matches!(b, b'+' | b'-') {
            if after_sign {
                return false;
            }
            after_sign = true;
        } else if b != b' ' && b != b'\t' {
            after_sign = false;
        }
        if b.is_ascii_digit() || (run > 0 && (b == b',' || b == b'.')) {
            run += 1;
            if run > MAX_NUMBER_CHARS {
                return false;
            }
        } else {
            run = 0;
        }
    }
    true
}

/// What a `plugin` line says (spec/beancount/auto-accounts.md).
enum PluginLine<'a> {
    /// The keyword with nothing after it.
    Bare,
    /// A module name as written between the quotes, and whether a configuration string follows.
    Named { module: &'a str, config: bool },
    /// Any other form: the reference rejects it and so does the parser, which reports it.
    Malformed,
}

/// The text of a string whose opening quote is already cut off, as written (escapes are not
/// read: a backslash makes the next character part of the string, so `\"` does not end it), and
/// what follows the closing quote.
fn string_at(rest: &str) -> Option<(&str, &str)> {
    let mut chars = rest.char_indices();
    while let Some((at, c)) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '"' => return Some((rest.get(..at)?, rest.get(at + 1..)?)),
            _ => {}
        }
    }
    None
}

/// Whether only blanks and a `;` comment are left on the line.
fn nothing_more(rest: &str) -> bool {
    let rest = rest.trim_start_matches([' ', '\t']);
    rest.is_empty() || rest.starts_with([';', '\n', '\r'])
}

/// Read a `plugin` line. `from_line` is the text from the start of the line to the end of the
/// file: a string may run over several lines, and the name is then written with them.
fn plugin_line(from_line: &str) -> PluginLine<'_> {
    let rest = from_line.strip_prefix("plugin").unwrap_or(from_line);
    if nothing_more(rest) {
        return PluginLine::Bare;
    }
    let Some((module, rest)) = rest
        .trim_start_matches([' ', '\t'])
        .strip_prefix('"')
        .and_then(string_at)
    else {
        return PluginLine::Malformed;
    };
    if nothing_more(rest) {
        return PluginLine::Named {
            module,
            config: false,
        };
    }
    match rest
        .trim_start_matches([' ', '\t'])
        .strip_prefix('"')
        .and_then(string_at)
    {
        Some((_, rest)) if nothing_more(rest) => PluginLine::Named {
            module,
            config: true,
        },
        _ => PluginLine::Malformed,
    }
}

/// What `prescan` found: the lines to refuse, whether the `auto_accounts` plugin is asked for and
/// how many `plugin` lines name `implicit_prices` (each one runs it).
pub(super) struct Prescan {
    pub refusals: Vec<Refusal>,
    pub auto_accounts: bool,
    pub implicit_prices: usize,
}

/// Lines to refuse, found without the parser.
pub(super) fn prescan(text: &str) -> Prescan {
    let mut out = Vec::new();
    let mut wanted = false;
    let mut implicit_runs = 0usize;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut offset = 0;
    for (index, raw) in text.split('\n').enumerate() {
        let line = u32::try_from(index + 1).unwrap_or(u32::MAX);
        let from_line = text.get(offset..).unwrap_or_default();
        offset += raw.len() + 1;
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        let refuse = |construct, name| Refusal {
            line,
            construct,
            name,
        };
        if starts_with_keyword(raw, "include") {
            out.push(refuse("include", None));
        } else if starts_with_keyword(raw, "plugin") {
            // Each `implicit_prices` line runs it again; `auto_accounts` runs once however often it
            // is named. Any configuration string makes the reference fail to apply the plugin.
            match plugin_line(from_line) {
                PluginLine::Named {
                    module: auto_accounts::MODULE,
                    config: false,
                } => wanted = true,
                PluginLine::Named {
                    module: implicit_prices::MODULE,
                    config: false,
                } => implicit_runs += 1,
                PluginLine::Named { module, .. } => {
                    out.push(refuse("plugin", Some(module.to_string())));
                }
                PluginLine::Bare => out.push(refuse("plugin", None)),
                // The parser gives the syntax error, on the line the reference gives it.
                PluginLine::Malformed => {}
            }
        } else if starts_with_keyword(raw, "option") {
            match option_name(raw) {
                Some(name) if HARMLESS_OPTIONS.contains(&name.as_str()) => {}
                name => out.push(refuse("option", name)),
            }
        } else if plainly_fine(raw) {
            // Nothing to refuse: the checks below would find nothing.
        } else if raw.starts_with([' ', '\t']) {
            let code = code_of(raw);
            if longest_number(&code) > MAX_NUMBER_CHARS {
                out.push(refuse("number-too-large", None));
            } else if has_partial_cost(&code) {
                out.push(refuse("cost-partial", None));
            } else if unreadable_sign(&code) {
                out.push(refuse("number-expression", None));
            }
        } else {
            let code = code_of(raw);
            if longest_number(&code) > MAX_NUMBER_CHARS {
                out.push(refuse("number-too-large", None));
            } else if is_amount_header(&code) && unreadable_sign(&code) {
                out.push(refuse("number-expression", None));
            }
        }
    }
    Prescan {
        refusals: out,
        auto_accounts: wanted,
        implicit_prices: implicit_runs,
    }
}

/// Whether a sign stands in front of another (`--4`, `+-6`): the reference reads that, the parser
/// reads one sign only. Or a `~` tolerance that is a sum (`~ 1 + 2`): the parser reads a plain
/// number there. Lines with `( ) / *` are read by `expr.rs`, which reads both, so they do not
/// count.
fn unreadable_sign(code: &str) -> bool {
    if code.contains(['(', ')', '/', '*']) {
        return false;
    }
    let bytes = code.as_bytes();
    let signs = bytes
        .iter()
        .enumerate()
        .filter(|(_, b)| matches!(b, b'+' | b'-'))
        .map(|(i, _)| i);
    for at in signs {
        // A sign of an operand starts a word: not `Assets:A--B`, not the `-` of `2026-01-02`.
        let before = bytes[..at].iter().rposition(|b| !matches!(b, b' ' | b'\t'));
        let glued = at > 0 && !matches!(bytes[at - 1], b' ' | b'\t' | b'{' | b'@' | b'~' | b',');
        let after = bytes[at + 1..]
            .iter()
            .find(|b| !matches!(b, b' ' | b'\t'))
            .copied();
        let binary = before.is_some_and(|i| matches!(bytes[i], b'0'..=b'9' | b'.'));
        if !glued && !binary && matches!(after, Some(b'+' | b'-')) {
            return true;
        }
    }
    // `~ 1 + 2`: an operator after the first character of the tolerance.
    if let Some(tilde) = code.find('~') {
        let tolerance: String = code[tilde + 1..]
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '+' | '-' | ' ' | '\t'))
            .collect();
        if tolerance
            .trim_start()
            .chars()
            .skip(1)
            .any(|c| matches!(c, '+' | '-'))
        {
            return true;
        }
    }
    false
}

/// A directive line that carries an amount: `balance` and `price`.
fn is_amount_header(code: &str) -> bool {
    matches!(code.split_whitespace().nth(1), Some("balance" | "price"))
}

fn is_flag_letter(c: char) -> bool {
    c.is_ascii_uppercase()
}

/// A transaction line whose flag is a bare capital letter: `2026-01-02 P "..."`.
fn header_flag_at(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let date = bytes.get(..10)?;
    let shaped = date.iter().enumerate().all(|(i, b)| match i {
        4 | 7 => *b == b'-',
        _ => b.is_ascii_digit(),
    });
    let letter = line.get(11..)?.chars().next()?;
    let after = line.get(12..)?.chars().next();
    (shaped
        && bytes.get(10) == Some(&b' ')
        && is_flag_letter(letter)
        && after.is_none_or(|c| c == ' ' || c == '\t'))
    .then_some(11)
}

/// A posting line whose flag is a bare capital letter: `  P Assets:Cash  1 EUR`.
fn posting_flag_at(line: &str) -> Option<usize> {
    let start = line.len() - line.trim_start().len();
    let rest = line.get(start..)?;
    let mut chars = rest.chars();
    let letter = chars.next()?;
    let gap = chars.next()?;
    let account = rest.get(1..)?.trim_start();
    let first_word = account.split([' ', '\t']).next()?;
    (is_flag_letter(letter)
        && (gap == ' ' || gap == '\t')
        && account.starts_with(|c: char| c.is_ascii_uppercase())
        && first_word.contains(':'))
    .then_some(start)
}

/// `{{}}` as `{}`, and a comma right after `{` dropped (`{ , "label"}`), outside strings and
/// comments: the reference reads both, the parser neither.
fn braces_tidied(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let (mut in_string, mut escaped) = (false, false);
    let mut skip_to = 0;
    for (i, c) in line.char_indices() {
        if i < skip_to {
            continue;
        }
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            ';' => {
                out.push_str(&line[i..]);
                break;
            }
            '{' => {
                let rest = &line[i + 1..];
                let trimmed = rest.trim_start();
                let empty_total = trimmed
                    .strip_prefix('{')
                    .and_then(|r| r.trim_start().strip_prefix("}}"));
                let after_comma = trimmed
                    .strip_prefix(',')
                    .filter(|r| !r.trim_start().starts_with('}'));
                if let Some(left) = empty_total {
                    out.push_str("{}");
                    skip_to = i + 1 + rest.len() - left.len();
                } else if let Some(left) = after_comma {
                    out.push('{');
                    skip_to = i + 1 + rest.len() - left.len();
                } else {
                    out.push(c);
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// `{{ total }}` as `{# total}`, outside strings and comments.
fn single_braces(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_string = false;
    let mut escaped = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            ';' => {
                out.push(c);
                out.extend(chars.by_ref());
            }
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                out.push_str("{# ");
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                out.push('}');
            }
            _ => out.push(c),
        }
    }
    out
}

/// An indented line of two or more tags and links, `  #a ^b`: the parser reads one per line.
/// They change nothing ledgermill reports, so the line becomes a comment.
fn is_tag_line(line: &str) -> bool {
    // The first token has to be a tag or a link.
    if !line.trim_start().starts_with(['#', '^']) {
        return false;
    }
    let code = line.split(';').next().unwrap_or("");
    let mut count = 0;
    for token in code.split_whitespace() {
        if !(token.len() > 1 && token.starts_with(['#', '^'])) {
            return false;
        }
        count += 1;
    }
    count >= 2
}

/// `5.` as `5`: the reference reads a number that ends in a dot, the parser does not.
///
/// On a directive line (`strict`) only a dot followed by a space or the end counts: a currency
/// such as `X1.Y` has a dot after a digit too.
fn trailing_dots_dropped(line: &str, strict: bool) -> Cow<'_, str> {
    // Most lines have no dot that ends a number: a dot after a digit and before another digit
    // (`12.50`) never does, so leave those alone without a closer look.
    let bytes = line.as_bytes();
    let candidate = bytes.iter().enumerate().any(|(i, &b)| {
        b == b'.'
            && i > 0
            && bytes.get(i - 1).is_some_and(u8::is_ascii_digit)
            && !bytes.get(i + 1).is_some_and(u8::is_ascii_digit)
    });
    if !candidate {
        return Cow::Borrowed(line);
    }
    // Copied only once a dot is dropped: most lines keep every character.
    let mut out: Option<String> = None;
    let mut in_string = false;
    let mut escaped = false;
    let mut in_comment = false;
    let mut previous = ' ';
    let mut chars = line.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        let ends_number = if strict {
            chars.peek().is_none_or(|(_, next)| next.is_whitespace())
        } else {
            !chars.peek().is_some_and(|(_, next)| next.is_ascii_digit())
        };
        let skip =
            !in_string && !in_comment && c == '.' && previous.is_ascii_digit() && ends_number;
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
        } else if c == '"' {
            in_string = true;
        } else if c == ';' {
            in_comment = true;
        }
        if skip {
            out.get_or_insert_with(|| line[..at].to_string());
        } else if let Some(out) = out.as_mut() {
            out.push(c);
        }
        previous = c;
    }
    out.map_or(Cow::Borrowed(line), Cow::Owned)
}

/// Where the minus sign of a negative tolerance is on a `balance` line (`500 ~ -0.5 EUR`). The
/// reference reads it and a negative tolerance passes nothing (spec/beancount/balance-directive.md);
/// the parser does not read it, so `normalize` drops the sign and says which lines had one.
fn negative_tolerance_at(line: &str) -> Option<usize> {
    if line.starts_with([' ', '\t']) || !line.contains('~') {
        return None;
    }
    let code = line.split(';').next()?;
    if code.split_whitespace().nth(1) != Some("balance") {
        return None;
    }
    let after = code.get(code.find('~')? + 1..)?;
    let trimmed = after.trim_start();
    trimmed.starts_with('-').then(|| code.len() - trimmed.len())
}

/// Whether a margin line is a comment the reference skips: it starts with `*` and has anything
/// after it, or with `#` and a character that cannot start a tag after it. A lone `*` or `#` and
/// `#tag` are syntax errors there (spec/beancount/syntax.md).
fn is_margin_comment(line: &str) -> bool {
    let mut chars = line.chars();
    match (chars.next(), chars.next()) {
        (Some('*'), Some(_)) => true,
        (Some('#'), Some(next)) => {
            !(next.is_ascii_alphanumeric() || matches!(next, '_' | '-' | '.' | '/'))
        }
        _ => false,
    }
}

/// Lines that start with `*` or `#` at the margin and are not comments there: a lone `*` or `#`,
/// or `#` and a tag (the parser would take the line for the tags of the directive above it).
pub(super) fn margin_errors(text: &str) -> Vec<u32> {
    let mut out = Vec::new();
    let mut in_string = false;
    for (index, raw) in text.split('\n').enumerate() {
        let body = raw.strip_suffix('\r').unwrap_or(raw);
        let body = if index == 0 {
            body.strip_prefix('\u{feff}').unwrap_or(body)
        } else {
            body
        };
        if !in_string && body.starts_with(['*', '#']) && !is_margin_comment(body) {
            out.extend(u32::try_from(index + 1));
        }
        in_string = string_open_after(body, in_string);
    }
    out
}

/// The bytes that end a word of a line's code: blanks, the delimiters around amounts, and the `"`
/// and `;` that start a string or a comment.
const WORD_BREAKS: [bool; 256] = {
    let mut table = [false; 256];
    let breaks = b" \t{},@~()\";";
    let mut i = 0;
    while i < breaks.len() {
        table[breaks[i] as usize] = true;
        i += 1;
    }
    table
};

fn is_word_break(b: u8) -> bool {
    WORD_BREAKS[usize::from(b)]
}

/// Whether a word (no space, brace, comma, `@`, `~`, parenthesis or quote in it) is a one-letter
/// currency glued to what follows it. `next` is the byte after the word, `None` at the end of the
/// line, and `eof` says that end is the end of the file. Bytes, not chars: every byte looked for
/// is ASCII, which never occurs inside a multi-byte sequence.
fn is_glued_currency(word: &[u8], next: Option<u8>, eof: bool) -> bool {
    // A sign and a number may be glued to the currency (`-2X`, `1.5X`). A word that starts with
    // `#`, `^` or `/` (a tag, a link, a longer currency) has no capital here and is not one.
    let start = word
        .iter()
        .position(|c| !(c.is_ascii_digit() || matches!(c, b'+' | b'-' | b',' | b'.')))
        .unwrap_or(word.len());
    let Some((letter, tail)) = word.get(start..).and_then(<[u8]>::split_first) else {
        return false;
    };
    if !letter.is_ascii_uppercase() {
        return false;
    }
    let glued = match tail.first().copied().or(next) {
        // A space, a tab or the end of the line after the letter: read as is. The end of the file
        // is not a line end for the reference.
        None => eof,
        Some(b' ' | b'\t') => false,
        // Otherwise only a longer currency saves it: one that ends in a letter or a digit.
        Some(_) => !tail
            .iter()
            .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || b"'._-".contains(*c))
            .any(|c| c.is_ascii_uppercase() || c.is_ascii_digit()),
    };
    // Accounts and metadata keys have capitals in them that are not currencies; looked at last,
    // as it reads the whole word.
    glued && !word.contains(&b':')
}

/// One pass over a line, given whether a string was open when it began: whether a word of its
/// code (outside strings and the comment) is a glued one-letter currency, and whether a string is
/// open after it. `\r` is a word byte: it is not a space for a one-letter currency.
fn scan_line(line: &[u8], mut open: bool, eof: bool) -> (bool, bool) {
    let mut found = false;
    let mut escaped = false;
    let mut i = 0;
    while let Some(&c) = line.get(i) {
        if open {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                open = false;
            }
            i += 1;
        } else if c == b'"' {
            open = true;
            i += 1;
        } else if c == b';' {
            break;
        } else if is_word_break(c) {
            i += 1;
        } else {
            let rest = line.get(i..).unwrap_or_default();
            let len = rest
                .iter()
                .position(|&b| is_word_break(b))
                .unwrap_or(rest.len());
            let word = rest.get(..len).unwrap_or_default();
            found = found || is_glued_currency(word, rest.get(len).copied(), eof);
            i += len;
        }
    }
    (found, open)
}

/// Lines with a one-letter currency glued to the character after it, outside strings, comments,
/// and margin comment lines (spec/beancount/check-errors.md). The parser reads `{2 X}` and `X;`
/// where the reference says `Invalid token: 'X}'`.
pub(super) fn glued_currency_errors(text: &str) -> Vec<u32> {
    let mut out = Vec::new();
    let mut in_string = false;
    let last = text.bytes().filter(|&b| b == b'\n').count();
    for (index, raw) in text.split('\n').enumerate() {
        let raw = if index == 0 {
            raw.strip_prefix('\u{feff}').unwrap_or(raw)
        } else {
            raw
        };
        if !in_string && is_margin_comment(raw.strip_suffix('\r').unwrap_or(raw)) {
            in_string = string_open_after(raw, false);
            continue;
        }
        let (found, open) = scan_line(raw.as_bytes(), in_string, index == last);
        in_string = open;
        if found {
            out.extend(u32::try_from(index + 1));
        }
    }
    out
}

/// Whether the line leaves a string open (a string that spans lines), given whether one was open
/// when it began. `;` starts a comment only outside a string.
fn string_open_after(line: &str, mut open: bool) -> bool {
    // Nothing can open a string on a line without a quote.
    if !open && !line.contains('"') {
        return false;
    }
    let mut escaped = false;
    // Bytes, not chars: the three characters that matter are ASCII, which never occur inside a
    // multi-byte sequence.
    for c in line.bytes() {
        if open {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                open = false;
            }
        } else if c == b'"' {
            open = true;
        } else if c == b';' {
            break;
        }
    }
    open
}

/// The lines of the options that a later line sets again: the reference lets the last one count
/// and the parser calls a repeat an error. Only the options ledgermill accepts are looked at.
fn superseded_options(text: &str) -> HashSet<usize> {
    let mut last: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut all: Vec<(usize, String)> = Vec::new();
    for (index, raw) in text.split('\n').enumerate() {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        if starts_with_keyword(raw, "option")
            && let Some(name) = option_name(raw)
            && HARMLESS_OPTIONS.contains(&name.as_str())
        {
            last.insert(name.clone(), index);
            all.push((index, name));
        }
    }
    all.into_iter()
        .filter(|(index, name)| last.get(name) != Some(index))
        .map(|(index, _)| index)
        .collect()
}

/// The text rewritten into forms the parser reads, with every line kept where it was: bare
/// capital-letter flags get the parser's quote (`P` becomes `'P`), and `{{ total }}` becomes
/// `{# total}`. Both are accepted by the reference tool (spec/beancount/header-flags.md,
/// spec/beancount/lots-costs.md).
pub(super) fn normalize(text: &str) -> Normalized {
    let mut out = String::with_capacity(text.len() + 16);
    let mut expressions = Expressions::default();
    let mut negative_tolerance = HashSet::new();
    let superseded = superseded_options(text);
    let mut in_string = false;
    for (index, raw) in text.split_inclusive('\n').enumerate() {
        let (body, end) = match raw.strip_suffix('\n') {
            Some(body) => match body.strip_suffix('\r') {
                Some(body) => (body, "\r\n"),
                None => (body, "\n"),
            },
            None => (raw, ""),
        };
        // A comment line (or an option set again later) becomes a `;` comment of the same length.
        let was_in_string = in_string;
        in_string = string_open_after(body, in_string);
        if !was_in_string && (is_margin_comment(body) || superseded.contains(&index)) {
            out.push(';');
            out.push_str(body.get(1..).unwrap_or(""));
            out.push_str(end);
            continue;
        }
        let at = if body.starts_with([' ', '\t']) {
            posting_flag_at(body)
        } else {
            header_flag_at(body)
        };
        // Borrowed until a rewrite changes the line.
        let mut line: Cow<str> = match at.and_then(|i| Some((body.get(..i)?, body.get(i..)?))) {
            Some((before, after)) => Cow::Owned(format!("{before}'{after}")),
            None => Cow::Borrowed(body),
        };
        let indented = line.starts_with([' ', '\t']);
        // Arithmetic is cut out first: what is left is plain numbers.
        if Expressions::wants(&line) {
            let number = u32::try_from(index + 1).unwrap_or(u32::MAX);
            let replaced = if indented || is_amount_header(&line) {
                expressions.line(number, &line, was_in_string, |left| {
                    operator_left(left, indented)
                })
            } else if is_custom_header(&line) {
                Some(expr::blank_operators(&line, was_in_string))
            } else {
                None
            };
            if let Some(replaced) = replaced {
                line = Cow::Owned(replaced);
            }
        }
        if indented && line.contains('{') {
            line = Cow::Owned(braces_tidied(&line));
        }
        if indented && line.contains("{{") {
            line = Cow::Owned(single_braces(&line));
        }
        if line.contains('.')
            && let Cow::Owned(dropped) = trailing_dots_dropped(&line, !indented)
        {
            line = Cow::Owned(dropped);
        }
        if indented && is_tag_line(&line) {
            line = Cow::Owned(format!("  ;{line}"));
        }
        if let Some(at) = negative_tolerance_at(&line) {
            line.to_mut().replace_range(at..=at, " ");
            negative_tolerance.insert(u32::try_from(index + 1).unwrap_or(u32::MAX));
        }
        out.push_str(&line);
        out.push_str(end);
    }
    Normalized {
        text: out,
        expressions,
        negative_tolerance,
    }
}

/// The text the parser reads, and what was cut out of it before.
pub(super) struct Normalized {
    pub text: String,
    pub expressions: Expressions,
    /// The `balance` lines whose tolerance had a minus sign (`500 ~ -0.5 EUR`), which was dropped
    /// from the text: the parser does not read it. A tolerance that was an expression has its
    /// sign in its value and is not listed.
    pub negative_tolerance: HashSet<u32>,
}

/// Whether the code left of a line, with its arithmetic and strings blanked, still has an
/// operator or a parenthesis that no number owns. A posting's flag `*` (also straight before the
/// account) and the merge mark of a
/// cost (`{*}`, `, *}`) are not operators.
fn operator_left(code: &str, indented: bool) -> bool {
    let code = merge_marker_removed(code);
    let code = match code.trim_start().strip_prefix('*') {
        Some(rest)
            if indented
                && (rest.starts_with([' ', '\t'])
                    || rest.starts_with(|c: char| c.is_uppercase())) =>
        {
            rest.to_string()
        }
        _ => code,
    };
    code.contains(['(', ')', '/', '*'])
}

/// A `custom` directive: its values are never used, and each may be an expression.
fn is_custom_header(line: &str) -> bool {
    !line.starts_with([' ', '\t']) && line.split_whitespace().nth(1) == Some("custom")
}
