#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Error {
    AccountNotExist,
    InvalidStateRoot,
    InvalidRLP,
    InvalidKey,
    TrieError,
    TrieReConstructFailed,
    DBError,
}

impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::AccountNotExist => return write!(f, "AccountNotExist"),
            Error::InvalidStateRoot => return write!(f, "InvalidStateRoot"),
            Error::InvalidRLP => return write!(f, "InvalidRLP"),
            Error::InvalidKey => return write!(f, "InvalidKey"),
            Error::TrieError => return write!(f, "TrieError"),
            Error::TrieReConstructFailed => return write!(f, "TrieReConstructFailed"),
            Error::DBError => return write!(f, "DBError"),
        }
    }
}
