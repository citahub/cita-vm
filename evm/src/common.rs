extern crate hex;

use ethereum_types::*;
use std::u64;

pub fn to_word_size(size: u64) -> u64 {
    if size > u64::MAX - 31 {
        return u64::MAX / 32 + 1;
    }
    return (size + 31) / 32;
}

pub fn mem_gas_cost(size: u64, memory_gas: u64) -> u64 {
    let size = to_word_size(size);
    size * memory_gas + size * size / 512
}

#[inline]
pub fn get_sign(value: U256) -> (U256, bool) {
    let U256(arr) = value;
    let sign = arr[3].leading_zeros() == 0;
    (set_sign(value, sign), sign)
}

#[inline]
pub fn set_sign(value: U256, sign: bool) -> U256 {
    if sign {
        (!U256::zero() ^ value).overflowing_add(U256::one()).0
    } else {
        value
    }
}

#[inline]
pub fn u256_to_address(value: &U256) -> Address {
    Address::from(H256::from(value))
}

#[inline]
pub fn address_to_u256(value: Address) -> U256 {
    U256::from(&*H256::from(value))
}

#[inline]
pub fn bool_to_u256(val: bool) -> U256 {
    if val {
        U256::one()
    } else {
        U256::zero()
    }
}

#[inline]
pub fn u256_min(x: U256, y: U256) -> U256 {
    if x > y {
        y
    } else {
        x
    }
}

#[inline]
pub fn rpad(slice: Vec<u8>, l: u64) -> Vec<u8> {
    let slice_len = slice.len();
    if l <= slice.len() as u64 {
        slice
    } else {
        let mut padded: Vec<u8> = Vec::new();
        let mut part1 = Vec::from(slice);
        padded.append(&mut part1);
        let mut part2 = vec![0; l as usize - slice_len];
        padded.append(&mut part2);
        padded
    }
}

#[inline]
pub fn copy_data(source: &[u8], start: U256, size: U256) -> Vec<u8> {
    let source_len = U256::from(source.len());
    let s = u256_min(start, source_len);
    let e = u256_min(s + size, source_len);

    let data = &source[s.as_usize()..e.as_usize()];
    let data = rpad(Vec::from(data), size.low_u64());
    data.iter().cloned().collect()
}

pub fn clean_0x(s: &str) -> &str {
    if s.starts_with("0x") {
        &s[2..]
    } else {
        s
    }
}

pub fn hex_decode(s: &str) -> Result<Vec<u8>, hex::FromHexError> {
    let s = clean_0x(s);
    hex::decode(s)
}

pub fn hex_encode(s: Vec<u8>) -> String {
    hex::encode(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_memory_gas_cost() {
        assert_eq!(mem_gas_cost(32, 3), 3);
        assert_eq!(mem_gas_cost(128, 3), 12);
        assert_eq!(mem_gas_cost(129, 3), 15);
        assert_eq!(mem_gas_cost(1024, 3), 98);
    }
}
