use std::error;
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Error {
    OutOfBounds,
    OutOfGas,
    OutOfStack,
    OutOfCode,
    OutOfData,
    MutableCallInStaticContext,
    InvalidOpcode,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::OutOfBounds => return write!(f, "OutOfBounds"),
            Error::OutOfGas => return write!(f, "OutOfGas"),
            Error::OutOfStack => return write!(f, "OutOfStack"),
            Error::OutOfCode => return write!(f, "OutOfCode"),
            Error::OutOfData => return write!(f, "OutOfData"),
            Error::MutableCallInStaticContext => return write!(f, "MutableCallInStaticContext"),
            Error::InvalidOpcode => return write!(f, "InvalidOpcode"),
        };
    }
}

impl error::Error for Error {}
