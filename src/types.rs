use ethereum_types::{Address, H256, U256};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InterpreterType {
    EVM,
    RISCV,
}

impl Default for InterpreterType {
    fn default() -> InterpreterType {
        InterpreterType::EVM
    }
}

#[derive(Clone, Debug, Default)]
pub struct Contract {
    pub code_address: Address,
    pub code_data: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct InterpreterParams {
    pub origin: Address,   // Who send the transaction
    pub sender: Address,   // Who send the call
    pub receiver: Address, // Who receive the transaction or call
    pub address: Address,  // Which storage used

    pub value: U256,
    pub input: Vec<u8>,
    pub itype: InterpreterType,
    pub nonce: U256,
    pub gas_limit: u64,
    pub gas_price: U256,

    pub read_only: bool,
    pub contract: Contract,
    pub extra: H256,
    pub is_create: bool,
    pub disable_transfer_value: bool,
    pub depth: u64,
}

// Log is the data struct for LOG0...LOG4.
// The members are "Address: Address, Topics: Vec<H256>, Body: Vec<u8>"
#[derive(Clone, Debug)]
pub struct Log(pub Address, pub Vec<H256>, pub Vec<u8>);

#[derive(Clone, Debug)]
pub enum InterpreterResult {
    // Return data, remain gas, logs.
    Normal(Vec<u8>, u64, Vec<Log>),
    // Return data, remain gas
    Revert(Vec<u8>, u64),
    // Return data, remain gas, logs, contract address
    Create(Vec<u8>, u64, Vec<Log>, Address),
}

/// Transaction struct.
#[derive(Clone, Debug)]
pub struct Transaction {
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub nonce: U256,
    pub gas_limit: u64,
    pub gas_price: U256,
    pub input: Vec<u8>,
    pub itype: InterpreterType,
}

#[derive(Clone, Debug, Default)]
pub struct Context {
    pub gas_limit: u64,
    pub coinbase: Address,
    pub number: U256,
    pub timestamp: u64,
    pub difficulty: U256,
}
