use numext_fixed_hash::H160 as Address;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_derive::Deserialize;
use serde_json::Error;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::io::Read;

#[derive(Debug, PartialEq, Deserialize)]
pub struct CallCreates {}

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
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct Exec {
    #[serde(rename = "address")]
    pub address: Data20,

    #[serde(rename = "caller")]
    pub caller: Data20,

    #[serde(rename = "code")]
    pub code: String,

    #[serde(rename = "data")]
    pub data: String,

    #[serde(rename = "gas")]
    pub gas: String,

    #[serde(rename = "gasPrice")]
    pub gas_price: String,

    #[serde(rename = "origin")]
    pub origin: Data20,

    #[serde(rename = "value")]
    pub value: String,
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
pub struct Vm {
    #[serde(rename = "callcreates")]
    pub call_creates: Option<Vec<CallCreates>>,

    #[serde(rename = "env")]
    pub env: Env,

    #[serde(rename = "exec")]
    pub exec: Exec,

    #[serde(rename = "gas")]
    pub gas: Option<String>,

    #[serde(rename = "logs")]
    pub logs: Option<String>,

    #[serde(rename = "out")]
    pub out: Option<String>,

    #[serde(rename = "post")]
    pub post: Option<State>,

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
        let f = fs::File::open("/tmp/jsondata/VMTests/vmArithmeticTest/add0.json").unwrap();
        let t = Test::load(f).unwrap();
        assert!(t.0.contains_key("add0"));
        let v = &t.0["add0"];
        assert_eq!(
            v.env.current_coinbase,
            Address::from("0x2adc25665018aa1fe0e6bc666dac8fc2697ff9ba")
        );
        assert_eq!(
            v.exec.address,
            Address::from("0x0f572e5295c57f15886f9b263e2f6d2d6c7b5ec6")
        );
        assert_eq!(v.gas, Some(String::from("0x013874")));
        if let Some(data) = &v.post {
            assert_eq!(
                data.0[&Address::from("0x0f572e5295c57f15886f9b263e2f6d2d6c7b5ec6")].balance,
                String::from("0x0de0b6b3a7640000")
            )
        }
    }
}
