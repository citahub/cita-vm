extern crate trie_db;
extern crate hash_db;
extern crate hex;
extern crate rlp;

use std::marker::PhantomData;
use std::error::Error;
use std::fmt;

use trie_db::{
    ChildReference,
    DBValue,
    NodeCodec,
    NibbleSlice,
    node::Node,
};
use hash_db::Hasher;
use rlp::{Rlp, DecoderError, Prototype, RlpStream, NULL_RLP};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CodecError {
    DecodeErr(DecoderError),
    InvalidData,
}

impl Error for CodecError {
	fn description(&self) -> &str {
		"codec error"
	}
}

impl fmt::Display for CodecError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let printable = match *self {
            CodecError::DecodeErr(ref err) => format!("rlp decode {}", err),
            CodecError::InvalidData => format!("invalid data"),
        };
        write!(f, "{}", printable)
	}
}

impl From<rlp::DecoderError> for CodecError {
    fn from(error: rlp::DecoderError) -> Self {
        CodecError::DecodeErr(error)
    }
}

#[derive(Default, Clone)]
pub struct RLPNodeCodec<H: Hasher>(PhantomData<H>);

impl<H: Hasher> NodeCodec<H> for RLPNodeCodec<H> {
    type Error = CodecError;
  
    fn hashed_null_node() -> H::Out {
        H::hash(&[0u8][..])
    }
  
    fn decode(data: &[u8]) -> Result<Node, Self::Error> {
        if data == &[0] {
            return Ok(Node::Empty);
        }
        let r = Rlp::new(data);
        match r.prototype()? {
            Prototype::List(2) => {
                let rlp_key = r.at(0)?.data()?;
                let value = r.at(1)?.data()?;
                let (key, is_leaf) = NibbleSlice::from_encoded(rlp_key);

                if is_leaf {
                    Ok(Node::Leaf(key, &value))
                } else {
                    Ok(Node::Extension(key, &value))
                }
            },
            Prototype::List(17) => {
                let mut nodes: [Option<&[u8]>; 16] = 
                    [
                        None, None, None, None,
                        None, None, None, None,
                        None, None, None, None,
                        None, None, None, None,
                    ];
                for i in 0..16 {
                    if !r.at(i)?.is_empty() {
                        nodes[i] = Some(r.at(i)?.as_raw());
                    }
                };

                let value_node = if r.at(16)?.is_empty() { 
                    None 
                } else { 
                    Some(r.at(16)?.data()?) 
                };

                Ok(Node::Branch(nodes, value_node))
            },
            Prototype::Data(0) => Ok(Node::Empty),
            _ => Err(CodecError::InvalidData),
        }
    }

    fn try_decode_hash(data: &[u8]) -> Option<H::Out>{
        if data.len() == H::LENGTH {
            Some(H::hash(&data[..]))
        } else {
            None
        }
    }

    fn is_empty_node(data: &[u8]) -> bool {
        data == NULL_RLP
    }

    fn empty_node() -> Vec<u8> {
        NULL_RLP.to_vec()
    }

    fn leaf_node(partial: &[u8], value: &[u8]) -> Vec<u8> {
        let mut stream = RlpStream::new_list(2);
        stream.append(&&*partial);
        stream.append(&&*value);
        stream.out()
    }

    fn ext_node(partial: &[u8], child_ref: ChildReference<H::Out>) -> Vec<u8> {
        let mut stream = RlpStream::new_list(2);
        stream.append(&&*partial);
        match child_ref {
			ChildReference::Hash(h) => stream.append(&AsRef::<[u8]>::as_ref(h.as_ref())),
			ChildReference::Inline(inline_data, len) => stream.append(&AsRef::<[u8]>::as_ref(&inline_data.as_ref())[..len].as_ref()),
		};
        stream.out()
    }

    fn branch_node<I>(children: I, value: Option<DBValue>) -> Vec<u8>
 	where I: IntoIterator<Item=Option<ChildReference<H::Out>>> + Iterator<Item=Option<ChildReference<H::Out>>> {
         let mut stream = RlpStream::new();
         for child in children {
             match child {
                Some(ChildReference::Hash(h)) => stream.append_raw(h.as_ref(), 1),
                 Some(ChildReference::Inline(inline_data, len)) => stream.
                    append_raw(&AsRef::<[u8]>::as_ref(&inline_data.as_ref())[..len].as_ref(), 1),
                None => stream.append_empty_data(),
             };
         }
         match value {
             Some(value) => stream.append(&&*value),
             None => stream.append_empty_data(),
         };
         stream.out()
     }
}
