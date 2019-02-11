use super::err;
use super::interpreter;
use super::opcodes;
use ethereum_types::*;

pub trait DataProvider {
    fn get_balance(&mut self, address: &Address) -> U256;

    fn add_refund(&mut self, address: &Address, n: u64);
    fn sub_refund(&mut self, address: &Address, n: u64);
    fn get_refund(&self, address: &Address) -> u64;

    fn get_code_size(&mut self, address: &Address) -> u64;
    fn get_code(&mut self, address: &Address) -> Vec<u8>;
    fn get_code_hash(&mut self, address: &Address) -> H256;

    fn get_block_hash(&self, number: &U256) -> H256;

    fn get_storage(&mut self, address: &Address, key: &H256) -> H256;
    fn set_storage(&mut self, address: &Address, key: H256, value: H256);
    fn get_storage_origin(&self, address: &Address, key: &H256) -> H256;
    fn set_storage_origin(&mut self, address: &Address, key: H256, value: H256);

    fn selfdestruct(&mut self, address: &Address, refund_address: &Address);
    fn sha3(&self, input: &[u8]) -> H256;
    // is_empty returns whether the given account is empty. Empty
    // is defined according to EIP161 (balance = nonce = code = 0).
    fn is_empty(&mut self, address: &Address) -> bool;

    // call is a low-level function for
    //   OpCode::CALL
    //   OpCode::CALLCODE
    //   OpCode::DELEGATECALL
    //   OpCode::STATICCALL
    //   OpCode::CREATE
    //   OpCode::CREATE2
    fn call(
        &self,
        opcode: opcodes::OpCode,
        params: interpreter::InterpreterParams,
    ) -> (Result<interpreter::InterpreterResult, err::Error>);
}
