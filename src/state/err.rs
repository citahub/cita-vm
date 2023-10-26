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
            Error::Trie(e) => write!(f, "state trie: {}", e),
            Error::RLP(e) => write!(f, "state rlp: {}", e),
            Error::DB(e) => write!(f, "state db: {}", e),
            Error::NotFound => write!(f, "state: not found"),
            Error::BalanceError => write!(f, "state: balance error"),
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
