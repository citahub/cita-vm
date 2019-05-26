use numext_fixed_hash::H160 as Address;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_derive::Deserialize;
use serde_json::Error;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::io::Read;

#[derive(Debug, PartialEq, Deserialize)]
pub struct Env {
    #[serde(rename = "currentCoinbase")]
    pub current_coinbase: Data20,

    #[serde(rename = "currentDifficulty")]
    pub current_difficulty: String,

    #[serde(rename = "currentGasLimit")]
    pub current_gas_limit: String,

    #[serde(rename = "currentNumber")]
    pub current_number: String,

    #[serde(rename = "currentTimestamp")]
    pub current_timestamp: String,

    #[serde(rename = "previousHash")]
    pub previous_hash: String,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct Transaction {
    #[serde(rename = "data")]
    pub data: Vec<String>,

    #[serde(rename = "gasLimit")]
    pub gas_limit: Vec<String>,

    #[serde(rename = "gasPrice")]
    pub gas_price: String,

    #[serde(rename = "nonce")]
    pub nonce: String,

    #[serde(rename = "secretKey")]
    pub secret_key: String,

    #[serde(rename = "to")]
    pub to: String,

    #[serde(rename = "value")]
    pub value: Vec<String>,
}

#[derive(Debug, PartialEq, Deserialize, Clone)]
pub struct Account {
    pub balance: String,
    pub code: String,
    pub nonce: String,
    pub storage: BTreeMap<String, String>,
}

#[derive(Debug, PartialEq, Deserialize, Clone)]
pub struct State(pub BTreeMap<Data20, Account>);

impl IntoIterator for State {
    type Item = <BTreeMap<Data20, Account> as IntoIterator>::Item;
    type IntoIter = <BTreeMap<Data20, Account> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct PostData {
    #[serde(rename = "hash")]
    pub hash: String,

    #[serde(rename = "indexes")]
    pub indexes: BTreeMap<String, usize>,

    #[serde(rename = "logs")]
    pub logs: String,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct Post {
    #[serde(rename = "Byzantium")]
    pub byzantium: Option<Vec<PostData>>,

    #[serde(rename = "Constantinople")]
    pub constantinople: Option<Vec<PostData>>,

    #[serde(rename = "EIP150")]
    pub eip150: Option<Vec<PostData>>,

    #[serde(rename = "EIP158")]
    pub eip158: Option<Vec<PostData>>,

    #[serde(rename = "Frontier")]
    pub frontier: Option<Vec<PostData>>,

    #[serde(rename = "Homestead")]
    pub homestead: Option<Vec<PostData>>,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct Vm {
    #[serde(rename = "env")]
    pub env: Env,

    #[serde(rename = "transaction")]
    pub transaction: Transaction,

    #[serde(rename = "post")]
    pub post: Option<Post>,

    #[serde(rename = "pre")]
    pub pre: Option<State>,
}

/// Fixed length bytes (wrapper structure around H160).
#[derive(Debug, PartialEq, Eq, Default, Hash, Clone)]
pub struct Data20(Address);
struct Data20Visitor;

impl Data20 {
    pub fn new(data: Address) -> Self {
        Self(data)
    }
}

impl Ord for Data20 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for Data20 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(&Data20(other.0.clone())))
    }
}

impl Serialize for Data20 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(self.0.as_bytes()))
    }
}

impl<'de> Deserialize<'de> for Data20 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(Data20Visitor)
    }
}

impl<'de> Visitor<'de> for Data20Visitor {
    type Value = Data20;

    fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        formatter.write_str(stringify!(Data20))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value.len() == 2 + 20usize * 2 && (&value[0..2] == "0x" || &value[0..2] == "0X") {
            let data = Address::from_hex_str(&value[2..]).map_err(|_| {
                if value.len() > 12 {
                    E::custom(format!(
                        "invalid hexadecimal string: [{}..(omit {})..{}]",
                        &value[..6],
                        value.len() - 12,
                        &value[value.len() - 6..value.len()]
                    ))
                } else {
                    E::custom(format!("invalid hexadecimal string: [{}]", value))
                }
            })?;
            Ok(Data20::new(data))
        } else {
            if value.len() > 12 {
                Err(E::custom(format!(
                    "invalid format: [{}..(omit {})..{}]",
                    &value[..6],
                    value.len() - 12,
                    &value[value.len() - 6..value.len()]
                )))
            } else {
                Err(E::custom(format!("invalid format: [{}]", value)))
            }
        }
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(value.as_ref())
    }
}

impl From<Address> for Data20 {
    fn from(data: Address) -> Data20 {
        Data20::new(data)
    }
}

impl Into<Address> for Data20 {
    fn into(self) -> Address {
        self.0
    }
}

impl Into<Vec<u8>> for Data20 {
    fn into(self) -> Vec<u8> {
        self.0.as_bytes().to_vec()
    }
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct Test(BTreeMap<String, Vm>);

impl IntoIterator for Test {
    type Item = <BTreeMap<String, Vm> as IntoIterator>::Item;
    type IntoIter = <BTreeMap<String, Vm> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Test {
    pub fn load<R>(reader: R) -> Result<Self, Error>
    where
        R: Read,
    {
        serde_json::from_reader(reader)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[test]
    fn test_json_tests_parse() {
        let f = fs::File::open("/tmp/jsondata/GeneralStateTests/stArgsZeroOneBalance/addmodNonConst.json").unwrap();
        let t = Test::load(f).unwrap();
        assert!(t.0.contains_key("addmodNonConst"));
        let v = &t.0["addmodNonConst"];
        assert_eq!(
            v.env.current_coinbase,
            Data20::from(Address::from_hex_str("2adc25665018aa1fe0e6bc666dac8fc2697ff9ba").unwrap())
        );
    }
}
