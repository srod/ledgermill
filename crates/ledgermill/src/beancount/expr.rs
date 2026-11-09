//! Arithmetic in a number (spec/beancount/number-expressions.md): `(1 + 2) * 3`, `10 / 4`, `-(2 - 5)`.
//!
//! The parser crate evaluates an expression while it reads it, with `rust_decimal`: a division by
//! zero or a product that does not fit in 96 bits stops the program, and a quotient has 28 decimal
//! places instead of 28 digits. So an expression never reaches it. Before the parser runs, each
//! one is cut out of its line, evaluated here with `pydec`, and replaced by a placeholder number
//! that the reader looks up again (`placeholder`, `Expressions::value_of`).
//!
//! An expression is the longest run of digits, `.`, `+ - * /`, parentheses and blanks (and a `,`
//! between two digits) that starts where a number can start. It ends at the currency, a `~`, a
//! `#`, a brace or whatever else is not part of it, so `1 + 2 EUR` is `3 EUR` (the notes call this
//! "no parentheses needed").

use super::pydec::Num;

/// Digits in a placeholder after the point. A number written in a journal never has this many
/// places (22 characters at most are read, `text.rs`), so a decimal with this scale is a
/// placeholder and nothing else.
const PLACEHOLDER_PLACES: u32 = 28;

/// How deep the parentheses can go: the evaluation recurses once per level.
const MAX_DEPTH: usize = 100;

/// The text that stands for the `index`-th (from 1) evaluated expression of a file.
fn placeholder(index: usize) -> String {
    format!("0.{index:0width$}", width = PLACEHOLDER_PLACES as usize)
}

/// A reason an expression has no value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Failure {
    /// Not an expression. `at_end` says the text stopped where more was needed, and then the
    /// reference blames the line after the one that holds it.
    Syntax {
        at_end: bool,
    },
    DivisionByZero,
}

#[derive(Debug, Clone)]
enum Token {
    Number(Num),
    Plus,
    Minus,
    Star,
    Slash,
    Open,
    Close,
}

/// The number that starts at `bytes[0]`, and how long it is: digits with groups of three after a
/// comma, and a point that may end it (`5.`). `None` when the comma groups are not of three.
fn number(bytes: &[u8]) -> Option<(Num, usize)> {
    let digits = bytes
        .iter()
        .take_while(|b| b.is_ascii_digit() || **b == b',')
        .count();
    let (whole, rest) = bytes.split_at(digits);
    let whole = std::str::from_utf8(whole).ok()?;
    let mut groups = whole.split(',');
    groups.next().filter(|first| !first.is_empty())?;
    if groups.any(|group| group.len() != 3) {
        return None;
    }
    let mut text = whole.replace(',', "");
    let mut length = digits;
    if rest.first() == Some(&b'.') {
        length += 1;
        let places = rest[1..].iter().take_while(|b| b.is_ascii_digit()).count();
        text.push('.');
        text.push_str(std::str::from_utf8(&rest[1..=places]).ok()?);
        length += places;
    }
    Some((Num::parse(&text)?, length))
}

fn tokens(text: &[u8]) -> Option<Vec<Token>> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(&byte) = text.get(at) {
        let token = match byte {
            b' ' | b'\t' => {
                at += 1;
                continue;
            }
            b'+' => Token::Plus,
            b'-' => Token::Minus,
            b'*' => Token::Star,
            b'/' => Token::Slash,
            b'(' => Token::Open,
            b')' => Token::Close,
            b'0'..=b'9' => {
                let (value, length) = number(&text[at..])?;
                at += length;
                out.push(Token::Number(value));
                continue;
            }
            _ => return None,
        };
        at += 1;
        out.push(token);
    }
    Some(out)
}

#[derive(Debug)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug)]
enum Tree {
    Number(Num),
    /// A sign in front of an operand; `true` for `-`.
    Sign(bool, Box<Tree>),
    Binary(Op, Box<Tree>, Box<Tree>),
}

/// Recursive descent: `sum = product (("+" | "-") product)*`, `product = operand (("*" | "/")
/// operand)*`, `operand = ("+" | "-") operand | number | "(" sum ")"`. Every operator goes from
/// the left and a sign belongs to the operand after it.
struct Reader {
    tokens: Vec<Token>,
    at: usize,
}

impl Reader {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }

    fn sum(&mut self, depth: usize) -> Result<Tree, Failure> {
        let mut left = self.product(depth)?;
        loop {
            let op = match self.peek() {
                Some(Token::Plus) => Op::Add,
                Some(Token::Minus) => Op::Sub,
                _ => return Ok(left),
            };
            self.at += 1;
            let right = self.product(depth)?;
            left = Tree::Binary(op, Box::new(left), Box::new(right));
        }
    }

    fn product(&mut self, depth: usize) -> Result<Tree, Failure> {
        let mut left = self.operand(depth)?;
        loop {
            let op = match self.peek() {
                Some(Token::Star) => Op::Mul,
                Some(Token::Slash) => Op::Div,
                _ => return Ok(left),
            };
            self.at += 1;
            let right = self.operand(depth)?;
            left = Tree::Binary(op, Box::new(left), Box::new(right));
        }
    }

    fn operand(&mut self, depth: usize) -> Result<Tree, Failure> {
        if depth > MAX_DEPTH {
            return Err(Failure::Syntax { at_end: false });
        }
        let Some(token) = self.tokens.get(self.at).cloned() else {
            return Err(Failure::Syntax { at_end: true });
        };
        self.at += 1;
        match token {
            Token::Number(value) => Ok(Tree::Number(value)),
            Token::Plus | Token::Minus => {
                let inner = self.operand(depth + 1)?;
                Ok(Tree::Sign(matches!(token, Token::Minus), Box::new(inner)))
            }
            Token::Open => {
                let inner = self.sum(depth + 1)?;
                match self.peek() {
                    Some(Token::Close) => {
                        self.at += 1;
                        Ok(inner)
                    }
                    Some(_) => Err(Failure::Syntax { at_end: false }),
                    None => Err(Failure::Syntax { at_end: true }),
                }
            }
            Token::Star | Token::Slash | Token::Close => Err(Failure::Syntax { at_end: false }),
        }
    }
}

fn evaluate(tree: &Tree) -> Result<Num, Failure> {
    match tree {
        Tree::Number(value) => Ok(value.clone()),
        Tree::Sign(negative, inner) => {
            let value = evaluate(inner)?;
            // Either sign of a zero gives a plain zero.
            Ok(if *negative {
                value.negated()
            } else {
                value.negated().negated()
            })
        }
        Tree::Binary(op, left, right) => {
            let (left, right) = (evaluate(left)?, evaluate(right)?);
            match op {
                Op::Add => Ok(left.add(&right)),
                Op::Sub => Ok(left.sub(&right)),
                Op::Mul => Ok(left.mul(&right)),
                Op::Div => left.div(&right).ok_or(Failure::DivisionByZero),
            }
        }
    }
}

/// The value of an expression written as text.
fn value_of(text: &[u8]) -> Result<Num, Failure> {
    let tokens = tokens(text).ok_or(Failure::Syntax { at_end: false })?;
    let mut reader = Reader { tokens, at: 0 };
    let tree = reader.sum(0)?;
    if reader.at < reader.tokens.len() {
        return Err(Failure::Syntax { at_end: false });
    }
    evaluate(&tree)
}

/// A line's text with its strings and its comment blanked, byte for byte; whether a string is
/// open at its end, given whether one was open at its start.
fn masked(line: &str, mut open: bool) -> String {
    let mut out = String::with_capacity(line.len());
    let mut escaped = false;
    let mut comment = false;
    for c in line.chars() {
        let keep = if comment {
            false
        } else if open {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                open = false;
            }
            false
        } else if c == '"' {
            open = true;
            false
        } else if c == ';' {
            comment = true;
            false
        } else {
            true
        };
        if keep {
            out.push(c);
        } else {
            out.extend(std::iter::repeat_n(' ', c.len_utf8()));
        }
    }
    out
}

fn is_blank(b: u8) -> bool {
    b == b' ' || b == b'\t'
}

/// Whether a run may go on at `at`: a comma only between two digits.
fn continues(text: &[u8], at: usize) -> bool {
    match text.get(at) {
        Some(b',') => {
            at > 0
                && text.get(at - 1).is_some_and(u8::is_ascii_digit)
                && text.get(at + 1).is_some_and(u8::is_ascii_digit)
        }
        Some(&b) => {
            b.is_ascii_digit()
                || matches!(b, b'.' | b'+' | b'-' | b'*' | b'/' | b'(' | b')')
                || is_blank(b)
        }
        None => false,
    }
}

/// Whether a number can start at `at`: after a blank, a brace, `@`, `~`, a comma, or a
/// parenthesis after `#`. Not inside an account, a currency or a tag.
fn can_start(text: &[u8], at: usize) -> bool {
    let Some(&first) = text.get(at) else {
        return false;
    };
    if !(first.is_ascii_digit() || matches!(first, b'(' | b'+' | b'-')) {
        return false;
    }
    match at.checked_sub(1).and_then(|before| text.get(before)) {
        None => true,
        Some(&before) => {
            is_blank(before)
                || matches!(before, b'{' | b'@' | b'~' | b',')
                || (before == b'#' && first == b'(')
        }
    }
}

/// The length of a date (`2026-01-02`) written at `at`, which is not a number.
fn date_len(text: &[u8], at: usize) -> Option<usize> {
    let date = text.get(at..at + 10)?;
    let shaped = date.iter().enumerate().all(|(i, b)| match i {
        4 | 7 => *b == b'-',
        _ => b.is_ascii_digit(),
    });
    (shaped && !text.get(at + 10).is_some_and(u8::is_ascii_digit)).then_some(10)
}

/// Whether a run is an expression and not a number: it has an operator or a parenthesis, not
/// counting a sign in front.
fn is_expression(run: &[u8]) -> bool {
    let body = run.iter().position(|b| !is_blank(*b)).unwrap_or(run.len());
    run.iter()
        .skip(body + 1)
        .any(|b| matches!(b, b'+' | b'-' | b'*' | b'/' | b'(' | b')'))
        || run
            .first()
            .is_some_and(|b| matches!(b, b'(' | b')' | b'*' | b'/'))
}

/// A line with its parentheses and operators blanked outside strings and comments. For a line
/// whose numbers are never used (`custom`), so that the parser has nothing to evaluate.
pub(super) fn blank_operators(line: &str, open: bool) -> String {
    let code = masked(line, open);
    line.char_indices()
        .map(|(at, c)| match code.as_bytes().get(at) {
            Some(b'(' | b')' | b'/' | b'*') => ' ',
            _ => c,
        })
        .collect()
}

/// What was found in a file: the values behind the placeholders, and the lines that cannot be
/// read.
#[derive(Debug, Default)]
pub(super) struct Expressions {
    values: Vec<Num>,
    /// Lines with an expression that is not one, and the line the reference blames.
    pub syntax: Vec<u32>,
    /// Lines with a division by zero.
    pub zero_division: Vec<u32>,
    /// Lines with a `( ) / *` left that is not part of a number we cut out.
    pub unsupported: Vec<u32>,
}

impl Expressions {
    /// The value of a decimal as the parser read it, when it is one of our placeholders.
    pub(super) fn value_of(&self, mantissa: i128, scale: u32) -> Option<Num> {
        if scale != PLACEHOLDER_PLACES {
            return None;
        }
        let index = usize::try_from(mantissa).ok()?.checked_sub(1)?;
        self.values.get(index).cloned()
    }

    /// Whether a line may hold an expression, and so is read by `line`.
    pub(super) fn wants(line: &str) -> bool {
        line.bytes().any(|b| matches!(b, b'(' | b')' | b'/' | b'*'))
    }

    /// The line with each expression replaced by a placeholder, or `None` when there is none.
    /// `open` says a string is open at the start of the line; `leftover` tells whether a
    /// `( ) / *` that remains in what is left of the code is not a plain flag or merge mark
    /// (the same for the whole line).
    pub(super) fn line(
        &mut self,
        number: u32,
        line: &str,
        open: bool,
        leftover: impl Fn(&str) -> bool,
    ) -> Option<String> {
        let code = masked(line, open);
        let text = code.as_bytes();
        let mut out = String::new();
        let mut done = 0;
        let mut blanked = code.clone().into_bytes();
        let mut at = 0;
        while at < text.len() {
            if !can_start(text, at) {
                at += 1;
                continue;
            }
            if let Some(length) = date_len(text, at) {
                at += length;
                continue;
            }
            let mut stop = at;
            while continues(text, stop) {
                stop += 1;
            }
            let mut end = stop;
            while end > at && text.get(end - 1).is_some_and(|b| is_blank(*b)) {
                end -= 1;
            }
            if !is_expression(&text[at..end]) {
                at = stop.max(at + 1);
                continue;
            }
            let replacement = match value_of(&text[at..end]) {
                Ok(value) => {
                    self.values.push(value);
                    placeholder(self.values.len())
                }
                Err(Failure::Syntax { at_end }) => {
                    let rest_is_blank = text[end..].iter().all(|b| is_blank(*b));
                    let blamed = if at_end && rest_is_blank {
                        number + 1
                    } else {
                        number
                    };
                    // One error per line, as the reference prints.
                    if self.syntax.last() != Some(&blamed) {
                        self.syntax.push(blamed);
                    }
                    "0".to_string()
                }
                Err(Failure::DivisionByZero) => {
                    self.zero_division.push(number);
                    "0".to_string()
                }
            };
            out.push_str(&line[done..at]);
            out.push_str(&replacement);
            done = end;
            blanked[at..end].fill(b' ');
            at = stop.max(at + 1);
        }
        let rest = String::from_utf8(blanked).unwrap_or_default();
        if leftover(&rest) {
            self.unsupported.push(number);
        }
        if done == 0 && out.is_empty() {
            return None;
        }
        out.push_str(&line[done..]);
        Some(out)
    }
}
