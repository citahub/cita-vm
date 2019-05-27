#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Trie(String),
    RLP(rlp::DecoderError),
    DB(String),
    NotFound,
    BalanceError,
}

impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Trie(e) => return write!(f, "state trie: {}", e),
            Error::RLP(e) => return write!(f, "state rlp: {}", e),
            Error::DB(e) => return write!(f, "state db: {}", e),
            Error::NotFound => return write!(f, "state: not found"),
            Error::BalanceError => return write!(f, "state: balance error"),
        }
    }
}

impl From<cita_trie::errors::TrieError> for Error {
    fn from(error: cita_trie::errors::TrieError) -> Self {
        Error::Trie(format!("{}", error))
    }
}

impl From<rlp::DecoderError> for Error {
    fn from(error: rlp::DecoderError) -> Self {
        Error::RLP(error)
    }
}
