//! Character classes, as the reference tool draws them (see `spec/hledger/numbers.md`,
//! `amount-forms.md` and `postings.md`).

/// Whitespace: ASCII space and tab to carriage return, the no-break space, and the Unicode
/// space separators. Not U+0085, U+2028 or the zero-width space: those are ordinary characters.
pub(crate) fn is_blank(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{202f}' | '\u{205f}' | '\u{3000}'
    )
}

pub(crate) fn trim_blank(text: &str) -> &str {
    text.trim_matches(is_blank)
}

/// Characters an unquoted commodity symbol is made of. Digits end it, as do a few characters
/// that mean something around an amount, and only the plain space, tab and newline: a no-break
/// space or a carriage return becomes part of the symbol.
pub(crate) fn is_symbol_char(c: char) -> bool {
    !c.is_ascii_digit()
        && !matches!(
            c,
            ' ' | '\t' | '\n' | '-' | '+' | '.' | '@' | ';' | '"' | '{' | '}' | '=' | '*'
        )
}

/// Spaces that may group digits inside a number.
pub(crate) fn is_group_space(c: char) -> bool {
    matches!(
        c,
        ' ' | '\u{a0}'
            | '\u{2002}'
            | '\u{2003}'
            | '\u{2008}'
            | '\u{2009}'
            | '\u{202f}'
            | '\u{205f}'
    )
}
