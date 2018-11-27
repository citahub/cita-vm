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
    // set_storage_origin is unsafe but required for vm tests.
    fn set_storage_origin(&mut self, address: &Address, key: H256, value: H256);

    fn logs(&mut self, topics: Vec<H256>, data: &[u8]);
    fn suicide(&mut self, refund_address: &Address);
    fn sha3(&self, input: &[u8]) -> H256;
    // is_empty returns whether the given account is empty. Empty
    // is defined according to EIP161 (balance = nonce = code = 0).
    fn is_empty(&self, address: &Address) -> bool;
}
