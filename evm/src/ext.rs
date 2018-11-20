use super::err;
use ethereum_types::*;

pub trait DataProvider {
    /// balance returns address balance.
    fn balance(&self, address: &Address) -> Result<U256, err::Error>;
    /// extcodesize returns address code size.
    fn extcodesize(&self, address: &Address) -> Result<u64, err::Error>;
    /// extcode returns code at given address.
    fn extcode(&self, address: &Address) -> Result<&[u8], err::Error>;
    /// extcodehash returns code hash at given address
    fn extcodehash(&self, address: &Address) -> Result<H256, err::Error>;
    /// blockhash returns the hash of one of the 256 most recent complete blocks.
    fn blockhash(&mut self, number: &U256) -> H256;
    /// get_storage returns a value for given key.
    fn get_storage(&self, key: &H256) -> Result<H256, err::Error>;
    /// Stores a value for given key.
    fn set_storage(&mut self, key: H256, value: H256) -> Result<(), err::Error>;
    /// get_storage_origin returns the storage value for a given key
    /// if reversion happens on the current transaction.
    fn get_storage_origin(&self, key: &H256) -> Result<H256, err::Error>;
    /// log creates log entry with given topics and data
    fn log(&mut self, topics: Vec<H256>, data: &[u8]) -> Result<(), err::Error>;
    /// suicide should be called when contract commits suicide.
    /// Address to which funds should be refunded.
    fn suicide(&mut self, refund_address: &Address) -> Result<(), err::Error>;
    /// hash returns the hash of inputs.
    fn hash(&self, input: &[u8]) -> H256;
}

pub struct DataProviderMock {}

impl DataProvider for DataProviderMock {
    fn balance(&self, _: &Address) -> Result<U256, err::Error> {
        Ok(U256::zero())
    }
    fn extcodesize(&self, _: &Address) -> Result<u64, err::Error> {
        Ok(0)
    }
    fn extcode(&self, _: &Address) -> Result<&[u8], err::Error> {
        Ok(&[0u8][..])
    }
    fn extcodehash(&self, _: &Address) -> Result<H256, err::Error> {
        Ok(H256::zero())
    }
    fn blockhash(&mut self, _: &U256) -> H256 {
        H256::zero()
    }
    fn get_storage(&self, _: &H256) -> Result<H256, err::Error> {
        Ok(H256::zero())
    }
    fn set_storage(&mut self, _: H256, _: H256) -> Result<(), err::Error> {
        Ok(())
    }
    fn get_storage_origin(&self, _: &H256) -> Result<H256, err::Error> {
        Ok(H256::zero())
    }
    fn log(&mut self, _: Vec<H256>, _: &[u8]) -> Result<(), err::Error> {
        Ok(())
    }
    fn suicide(&mut self, _: &Address) -> Result<(), err::Error> {
        Ok(())
    }
    fn hash(&self, _: &[u8]) -> H256 {
        H256::zero()
    }
}
