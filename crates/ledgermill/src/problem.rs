//! What a parser can say about a line it did not accept: either the reference tool would reject
//! it too (`Invalid`), or it is something ledgermill does not read yet (`Unsupported`).

/// Something ledgermill does not read yet. The reference tool accepts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unsupported {
    Include,
    /// A date without a year and no `Y` directive: the reference tool uses the current year.
    DateWithoutYear,
    /// A carriage return inside a line.
    ControlCharacter,
    NumberTooLarge,
    /// Conversion postings and prices in a shape whose rule is not pinned down.
    ConversionPostings,
    /// A period expression whose verdict the spec notes do not settle.
    PeriodExpression,
    /// A timeclock file, read by the reference tool by its extension.
    Timeclock,
    /// A timedot file, read by the reference tool by its extension.
    Timedot,
    /// A CSV file, read by the reference tool by its extension (with rules of its own).
    Csv,
    /// A `P` line whose rate is followed by a price, a lot or a label.
    PriceAnnotation,
}

impl Unsupported {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Unsupported::Include => "include",
            Unsupported::DateWithoutYear => "date-without-year",
            Unsupported::ControlCharacter => "control-character",
            Unsupported::NumberTooLarge => "number-too-large",
            Unsupported::ConversionPostings => "conversion-postings",
            Unsupported::PeriodExpression => "period-expression",
            Unsupported::Timeclock => "timeclock",
            Unsupported::Timedot => "timedot",
            Unsupported::Csv => "csv",
            Unsupported::PriceAnnotation => "price-annotation",
        }
    }

    pub(crate) fn message(self) -> &'static str {
        match self {
            Unsupported::Include => "Including other files is not supported.",
            Unsupported::DateWithoutYear => {
                "A date without a year needs a Y directive before it to be read here."
            }
            Unsupported::ControlCharacter => "A carriage return inside a line is not supported.",
            Unsupported::NumberTooLarge => "This number is too large to handle.",
            Unsupported::ConversionPostings => {
                "Conversion postings in this arrangement are not supported yet."
            }
            Unsupported::PeriodExpression => {
                "This period expression is not one ledgermill can tell the reference tool's verdict on."
            }
            Unsupported::Timeclock => "Timeclock files are not supported.",
            Unsupported::Timedot => "Timedot files are not supported.",
            Unsupported::Csv => "CSV files are not supported.",
            Unsupported::PriceAnnotation => {
                "A price, a lot or a label after the rate of a P line is not supported."
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

/// The finding `Ledger::at_cost` and `Ledger::value` return for a question they cannot answer
/// as the reference tool does.
pub(crate) fn unsupported_finding(
    construct: &str,
    message: &str,
    file: &str,
    line: u32,
) -> crate::model::Finding {
    crate::model::Finding {
        code: "unsupported-syntax".into(),
        severity: crate::model::Severity::Error,
        file: file.into(),
        line,
        column: None,
        details: serde_json::json!({ "construct": construct }),
        message: message.into(),
    }
}
