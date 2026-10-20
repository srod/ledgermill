//! What a parser can say about a line it did not accept: either the reference tool would reject
//! it too (`Invalid`), or it is something ledgermill does not read yet (`Unsupported`).

/// Something ledgermill does not read yet. The reference tool accepts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unsupported {
    Directive,
    Include,
    BalanceAssertion,
    /// A date without a year and no `Y` directive: the reference tool uses the current year.
    DateWithoutYear,
    /// A carriage return inside a line.
    ControlCharacter,
    NumberTooLarge,
    /// Conversion postings and prices in a shape whose rule is not pinned down.
    ConversionPostings,
}

impl Unsupported {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Unsupported::Directive => "directive",
            Unsupported::Include => "include",
            Unsupported::BalanceAssertion => "balance-assertion",
            Unsupported::DateWithoutYear => "date-without-year",
            Unsupported::ControlCharacter => "control-character",
            Unsupported::NumberTooLarge => "number-too-large",
            Unsupported::ConversionPostings => "conversion-postings",
        }
    }

    pub(crate) fn message(self) -> &'static str {
        match self {
            Unsupported::Directive => "This directive is not supported yet.",
            Unsupported::Include => "Including other files is not supported.",
            Unsupported::BalanceAssertion => {
                "Balance assertions and assignments are not supported yet."
            }
            Unsupported::DateWithoutYear => {
                "A date without a year needs a Y directive before it to be read here."
            }
            Unsupported::ControlCharacter => "A carriage return inside a line is not supported.",
            Unsupported::NumberTooLarge => "This number is too large to handle.",
            Unsupported::ConversionPostings => {
                "Conversion postings in this arrangement are not supported yet."
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Problem {
    /// The reference tool rejects this; the text is a short English reason.
    Invalid(&'static str),
    Unsupported(Unsupported),
}

pub(crate) type Parsed<T> = Result<T, Problem>;

pub(crate) fn invalid<T>(reason: &'static str) -> Parsed<T> {
    Err(Problem::Invalid(reason))
}
