//! Reading the raw text around the parser: line numbers, and the lines that are refused before
//! the parser sees them (it would read the filesystem for `include`, and a few number
//! expressions would make it panic).

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
    /// The option name for `option`.
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

fn option_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("option")?;
    let start = rest.find('"')? + 1;
    let end = rest.get(start..)?.find('"')? + start;
    rest.get(start..end).map(str::to_string)
}

/// Lines to refuse, found without the parser.
pub(super) fn prescan(text: &str) -> Vec<Refusal> {
    let mut out = Vec::new();
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    for (index, raw) in text.split('\n').enumerate() {
        let line = u32::try_from(index + 1).unwrap_or(u32::MAX);
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        let refuse = |construct, name| Refusal {
            line,
            construct,
            name,
        };
        if starts_with_keyword(raw, "include") {
            out.push(refuse("include", None));
        } else if starts_with_keyword(raw, "plugin") {
            out.push(refuse("plugin", None));
        } else if starts_with_keyword(raw, "option") {
            match option_name(raw) {
                Some(name) if HARMLESS_OPTIONS.contains(&name.as_str()) => {}
                name => out.push(refuse("option", name)),
            }
        } else if raw.starts_with([' ', '\t']) {
            // A posting's flag may be `*`; it is not an operator.
            let code = merge_marker_removed(&code_of(raw));
            let code = match code.trim_start().strip_prefix('*') {
                Some(rest) if rest.starts_with([' ', '\t']) => rest.to_string(),
                _ => code,
            };
            if code.contains(['(', ')', '/', '*']) {
                out.push(refuse("number-expression", None));
            } else if longest_number(&code) > MAX_NUMBER_CHARS {
                out.push(refuse("number-too-large", None));
            }
        } else if longest_number(&code_of(raw)) > MAX_NUMBER_CHARS {
            out.push(refuse("number-too-large", None));
        }
    }
    out
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
    let code = line.split(';').next().unwrap_or("");
    let tokens: Vec<&str> = code.split_whitespace().collect();
    tokens.len() >= 2
        && tokens
            .iter()
            .all(|t| t.len() > 1 && t.starts_with(['#', '^']))
}

/// `5.` as `5`: the reference reads a number that ends in a dot, the parser does not.
fn trailing_dots_dropped(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_string = false;
    let mut escaped = false;
    let mut in_comment = false;
    let mut previous = ' ';
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        let next_is_digit = chars.peek().is_some_and(char::is_ascii_digit);
        let skip =
            !in_string && !in_comment && c == '.' && previous.is_ascii_digit() && !next_is_digit;
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
        if !skip {
            out.push(c);
        }
        previous = c;
    }
    out
}

/// The text rewritten into forms the parser reads, with every line kept where it was: bare
/// capital-letter flags get the parser's quote (`P` becomes `'P`), and `{{ total }}` becomes
/// `{# total}`. Both are accepted by the reference tool (spec/beancount/header-flags.md,
/// spec/beancount/lots-costs.md).
pub(super) fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    for raw in text.split_inclusive('\n') {
        let (body, end) = match raw.strip_suffix('\n') {
            Some(body) => match body.strip_suffix('\r') {
                Some(body) => (body, "\r\n"),
                None => (body, "\n"),
            },
            None => (raw, ""),
        };
        let at = if body.starts_with([' ', '\t']) {
            posting_flag_at(body)
        } else {
            header_flag_at(body)
        };
        let mut line = match at.and_then(|i| Some((body.get(..i)?, body.get(i..)?))) {
            Some((before, after)) => format!("{before}'{after}"),
            None => body.to_string(),
        };
        if line.starts_with([' ', '\t']) && line.contains("{{") {
            line = single_braces(&line);
        }
        if line.starts_with([' ', '\t']) && line.contains('.') {
            line = trailing_dots_dropped(&line);
        }
        if line.starts_with([' ', '\t']) && is_tag_line(&line) {
            line = format!("  ;{line}");
        }
        out.push_str(&line);
        out.push_str(end);
    }
    out
}
