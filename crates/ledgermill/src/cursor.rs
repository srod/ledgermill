//! A position in one line of text, for the hand-written parsers. It only ever moves by whole
//! characters, so slicing at its position is always valid.

#[derive(Debug, Clone, Copy)]
pub(crate) struct Cursor<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(text: &'a str) -> Cursor<'a> {
        Cursor { text, pos: 0 }
    }

    pub(crate) fn rest(&self) -> &'a str {
        self.text.get(self.pos..).unwrap_or("")
    }

    pub(crate) fn at_end(&self) -> bool {
        self.rest().is_empty()
    }

    pub(crate) fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    /// The character `n` places after the next one (`peek_nth(0)` is `peek()`).
    pub(crate) fn peek_nth(&self, n: usize) -> Option<char> {
        self.rest().chars().nth(n)
    }

    pub(crate) fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    pub(crate) fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.pos += c.len_utf8();
            true
        } else {
            false
        }
    }

    pub(crate) fn eat_str(&mut self, s: &str) -> bool {
        if self.rest().starts_with(s) {
            self.pos += s.len();
            true
        } else {
            false
        }
    }

    pub(crate) fn take_while(&mut self, pred: impl Fn(char) -> bool) -> &'a str {
        let rest = self.rest();
        let len = rest.find(|c| !pred(c)).unwrap_or(rest.len());
        self.pos += len;
        rest.get(..len).unwrap_or("")
    }

    /// Skip blanks other than a newline.
    pub(crate) fn skip_blanks(&mut self) {
        self.take_while(|c| crate::chars::is_blank(c) && c != '\n');
    }

    /// Skip whitespace (any kind); returns how many characters were skipped.
    pub(crate) fn skip_ws(&mut self) -> usize {
        self.take_while(char::is_whitespace).chars().count()
    }
}
