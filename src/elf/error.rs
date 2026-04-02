use crate::binary::BinaryError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParserError {
    /// Any invalid field, reason is a string (preformatted)
    #[error("'{field}' is invalid: {kind}")]
    InvalidField {
        field: &'static str,
        kind: FieldErrorKind,
    },

    /// Size mismatch (header, section, program header, etc.)
    #[error("invalid size for '{field}': got {found}, expected {expected}")]
    InvalidSize {
        field: &'static str,
        found: usize,
        expected: usize,
    },

    #[error("invalid relationship between fields: {fields:?} - {kind}")]
    Inconsistent {
        fields: &'static [&'static str],
        kind: InconsistentKind,
    },

    #[error("'{what}' is out of bounds: range [{start:#x}..{end:#x}) exceeds file size {size:#x}")]
    OutOfBounds {
        what: &'static str,
        start: usize,
        end: usize,
        size: usize,
    },

    #[error("arithmetic overflow during {op}: {lhs} and {rhs}")]
    Overflow {
        op: &'static str,
        lhs: usize,
        rhs: usize,
    },

    #[error("conversion falied for '{field}': {value} does not fit in {target}")]
    IntegerConversion {
        field: &'static str,
        value: u64,
        target: &'static str,
    },

    /// Wrap binary-level errors
    #[error(transparent)]
    Binary(#[from] BinaryError),
}

#[derive(Debug)]
pub enum FieldErrorKind {
    InvalidValue,
    NotEqual { expected: u64, found: u64 },
    NotInSet(&'static str),
    NotPowerOfTwo { value: u64 },
}

impl std::fmt::Display for FieldErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidValue => write!(f, "invalid value"),
            Self::NotEqual { expected, found } => {
                write!(f, "found {}, expected {}", found, expected)
            }
            Self::NotInSet(set) => write!(f, "value not in {}", set),
            Self::NotPowerOfTwo { value } => write!(f, "{} is not a power of two", value),
        }
    }
}

#[derive(Debug)]
pub enum InconsistentKind {
    OffsetWithoutCount { count: u64 },
    SizeMismatch { smaller: u64, larger: u64 },
    Misaligned { align: u64, a: u64, b: u64 },
}

impl std::fmt::Display for InconsistentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OffsetWithoutCount { count } => {
                write!(f, "offset is 0 but count is {}", count)
            }
            Self::SizeMismatch { smaller, larger } => {
                write!(f, "{} is smaller than {}", smaller, larger)
            }
            Self::Misaligned { align, a, b } => {
                write!(f, "0x{:X} and 0x{:X} are not {}-byte aligned", a, b, align)
            }
        }
    }
}
