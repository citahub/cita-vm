use std::error;
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Error {
    OutOfGas,
    OutOfBounds,
    Unknown,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::OutOfGas => return write!(f, "OutOfGas"),
            Error::OutOfBounds => return write!(f, "OutOfBounds"),
            Error::Unknown => return write!(f, "Unknown"),
        };
    }
}

impl error::Error for Error {}
