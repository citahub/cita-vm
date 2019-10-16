use std::u64;

use ethereum_types::{Address, H256, U256};

/// The size of word in EVM is 32 bytes.
#[inline]
pub fn to_word_size(size: u64) -> u64 {
    if size > u64::MAX - 31 {
        return (u64::MAX >> 5) + 1;
    }
    (size + 31) >> 5
}

/// Get the total gas of memory cost.
/// C_mem(a) ≡ G_memory * a + (a ^ 2 / 512); where unit of a is word.
#[inline]
pub fn mem_gas_cost(size: u64, memory_gas: u64) -> u64 {
    let size = to_word_size(size);
    size * memory_gas + ((size * size) >> 9)
}

/// Get the signed 256 from Unsigned 256.
#[inline]
pub fn get_sign(value: U256) -> (U256, bool) {
    // The highest bit is 1, indicating that it is negative
    let sign = (value >> 255) == U256::one();
    (set_sign(value, sign), sign)
}

/// Set the sign for a U256.
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
pub fn rpad(slice: Vec<u8>, n: usize) -> Vec<u8> {
    let slice_len = slice.len();
    if n <= slice.len() {
        slice
    } else {
        let mut padded: Vec<u8> = Vec::with_capacity(n);
        let mut part1 = slice;
        padded.append(&mut part1);
        let mut part2 = vec![0; n as usize - slice_len];
        padded.append(&mut part2);
        padded
    }
}

/// Copy data from source by start and size.
#[inline]
pub fn copy_data(source: &[u8], start: U256, size: U256) -> Vec<u8> {
    if size.is_zero() {
        return Vec::new();
    }
    let source_len = U256::from(source.len());
    let s = u256_min(start, source_len);
    let e = u256_min(s + size, source_len);

    let data = &source[s.as_usize()..e.as_usize()];
    rpad(Vec::from(data), size.as_usize())
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
