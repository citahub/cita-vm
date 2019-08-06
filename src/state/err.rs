#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    BalanceError,
    DB(String),
    NotFound,
    RLP(rlp::DecoderError),
    Trie(String),
}

impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::BalanceError => return write!(f, "BalanceError"),
            Error::DB(e) => return write!(f, "{}", e),
            Error::NotFound => return write!(f, "NotFound"),
            Error::RLP(e) => return write!(f, "{}", e),
            Error::Trie(e) => return write!(f, "{}", e),
        }
    }
}

impl From<cita_trie::TrieError> for Error {
    fn from(error: cita_trie::TrieError) -> Self {
        Error::Trie(format!("{}", error))
    }
}

impl From<rlp::DecoderError> for Error {
    fn from(error: rlp::DecoderError) -> Self {
        Error::RLP(error)
    }
}
