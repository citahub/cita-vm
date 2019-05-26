use numext_fixed_uint::U256;

pub fn u256_2_rlp256(value: U256) -> Vec<u8> {
    let leading_empty_bytes: u32 = value.leading_zeros() / 8;
    let mut buffer = [0u8; 32];
    let _ = value.into_big_endian(&mut buffer);
    buffer[leading_empty_bytes as usize..].to_vec()
}
