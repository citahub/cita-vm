use super::err;
use super::interpreter;
use super::opcodes;
use ethereum_types::*;

pub trait DataProvider {
    fn get_balance(&self, address: &Address) -> U256;

    fn add_refund(&mut self, n: u64);
    fn sub_refund(&mut self, n: u64);
    fn get_refund(&self) -> u64;

    fn get_code_size(&self, address: &Address) -> u64;
    fn get_code(&self, address: &Address) -> &[u8];
    fn get_code_hash(&self, address: &Address) -> H256;

    fn get_block_hash(&self, number: &U256) -> H256;

    fn get_storage(&self, address: &Address, key: &H256) -> H256;
    fn set_storage(&mut self, address: &Address, key: H256, value: H256);
    fn get_storage_origin(&self, address: &Address, key: &H256) -> H256;
    fn set_storage_origin(&mut self, address: &Address, key: H256, value: H256);

    fn suicide(&mut self, refund_address: &Address);
    fn sha3(&self, input: &[u8]) -> H256;
    // is_empty returns whether the given account is empty. Empty
    // is defined according to EIP161 (balance = nonce = code = 0).
    fn is_empty(&self, address: &Address) -> bool;

    // call is a low-level function for
    //   OpCode::CALL
    //   OpCode::CALLCODE
    //   OpCode::DELEGATECALL
    //   OpCode::STATICCALL
    //   OpCode::CREATE
    //   OpCode::CREATE2
    fn call(
        &self,
        contract_address: &Address, // For CALL, CALLCODE, DELEGATECALL and STATICCALL
        input: &[u8],
        gas: u64,
        value: U256,
        extra: U256, // Only used for CREATE2, as salt
        from: opcodes::OpCode,
    ) -> (Result<interpreter::InterpreterResult, err::Error>);
}
