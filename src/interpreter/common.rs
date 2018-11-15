use ethereum_types::U256;
use std::u64;

pub fn u256_bit_len(u: U256) -> u64 {
    u.bits() as u64
    // let array: [u8; 32] = u.into();
    // let mut nb_zero = 0;
    // for i in array.iter() {
    //     if *i == 0u8 {
    //         nb_zero += 1
    //     }
    // }
    // return 32 - nb_zero
}

pub fn to_word_size(size: u64) -> u64 {
    if size > u64::MAX - 31 {
        return u64::MAX / 32 + 1;
    }
    return (size + 31) / 32;
}


pub fn calc_mem_size(off: U256, l: U256) -> U256 {
    if l.is_zero() {
        return U256::zero()
    }
    off + l
}
