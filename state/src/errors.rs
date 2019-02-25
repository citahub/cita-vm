#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Trie(String),
    RLP(rlp::DecoderError),
    DB(String),
    KeyNotFound,
    NotInCache,
}

impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Trie(e) => return write!(f, "state trie: {}", e),
            Error::RLP(e) => return write!(f, "state rlp: {}", e),
            Error::DB(e) => return write!(f, "state db: {}", e),
            Error::KeyNotFound => return write!(f, "state: key not found"),
            Error::NotInCache => return write!(f, "state: key not in cache"),
        }
    }
}

impl<C: cita_trie::codec::NodeCodec, B: cita_trie::db::DB> From<cita_trie::errors::TrieError<C, B>>
    for Error
{
    fn from(error: cita_trie::errors::TrieError<C, B>) -> Self {
        Error::Trie(format!("{}", error))
    }
}
impl From<rlp::DecoderError> for Error {
    fn from(error: rlp::DecoderError) -> Self {
        Error::RLP(error)
    }
}
