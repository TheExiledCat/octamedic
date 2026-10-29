use std::fmt::Display;

#[derive(Debug)]
pub enum MmdError {
    Io(std::io::Error),
    /// The file doesn't start with an MMD0-MMD3 id.
    UnknownFormat(u32),
    /// A pointer the format requires is null.
    MissingPointer(&'static str),
    /// Valid MMD this crate can't read or write yet.
    Unsupported(&'static str),
    UnknownInstrumentType(i16),
}

pub type MmdResult<T> = Result<T, MmdError>;

impl Display for MmdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            MmdError::Io(error) => write!(f, "{error}"),
            MmdError::UnknownFormat(id) => write!(f, "not an MMD module (id {id:#010x})"),
            MmdError::MissingPointer(name) => write!(f, "the module has no {name}"),
            MmdError::Unsupported(what) => write!(f, "{what} aren't supported yet"),
            MmdError::UnknownInstrumentType(kind) => write!(f, "unknown instrument type {kind}"),
        };
    }
}

impl std::error::Error for MmdError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        return match self {
            MmdError::Io(error) => Some(error),
            _ => None,
        };
    }
}

impl From<std::io::Error> for MmdError {
    fn from(error: std::io::Error) -> Self {
        return MmdError::Io(error);
    }
}
