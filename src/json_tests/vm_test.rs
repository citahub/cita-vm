use std::collections::HashMap;
use std::io::Read;

use ethereum_types::Address;
use serde_derive::Deserialize;
use serde_json::Error;

#[derive(Debug, PartialEq, Deserialize)]
pub struct CallCreates {}

#[derive(Debug, PartialEq, Deserialize)]
pub struct Env {
    #[serde(rename = "currentCoinbase")]
    pub current_coinbase: Address,

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
    pub address: Address,

    #[serde(rename = "caller")]
    pub caller: Address,

    #[serde(rename = "code")]
    pub code: String,

    #[serde(rename = "data")]
    pub data: String,

    #[serde(rename = "gas")]
    pub gas: String,

    #[serde(rename = "gasPrice")]
    pub gas_price: String,

    #[serde(rename = "origin")]
    pub origin: Address,

    #[serde(rename = "value")]
    pub value: String,
}

#[derive(Debug, PartialEq, Deserialize, Clone)]
pub struct Account {
    pub balance: String,
    pub code: String,
    pub nonce: String,
    pub storage: HashMap<String, String>,
}

#[derive(Debug, PartialEq, Deserialize, Clone)]
pub struct State(pub HashMap<Address, Account>);

impl IntoIterator for State {
    type Item = <HashMap<Address, Account> as IntoIterator>::Item;
    type IntoIter = <HashMap<Address, Account> as IntoIterator>::IntoIter;

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

#[derive(Debug, PartialEq, Deserialize)]
pub struct Test(HashMap<String, Vm>);

impl IntoIterator for Test {
    type Item = <HashMap<String, Vm> as IntoIterator>::Item;
    type IntoIter = <HashMap<String, Vm> as IntoIterator>::IntoIter;

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
