use ethereum_types::*;

// use super::common::hasher::{Hasher, Sha3Hasher};
// use super::evm::{Contract, EVMContext};

/// 0s: Stop and Arithmetic Operations
pub fn stop(ctx: &mut EVMContext, contract: &Contract) {}

pub fn add(ctx: &mut EVMContext, contract: &Contract) {
    let a = ctx.stack.pop();
    let b = ctx.stack.pop();

    ctx.stack.push(a + b)
}

pub fn mul(ctx: &mut EVMContext, contract: &Contract) {
    let a = ctx.stack.pop();
    let b = ctx.stack.pop();

    ctx.stack.push(a * b)
}

pub fn sub(ctx: &mut EVMContext, contract: &Contract) {
    let a = ctx.stack.pop();
    let b = ctx.stack.pop();
    ctx.stack.push(a - b)
}

pub fn div(ctx: &mut EVMContext, contract: &Contract) {
    let a = ctx.stack.pop();
    let b = ctx.stack.pop();

    ctx.stack
        .push(if b.is_zero() { U256::zero() } else { a / b })
}

pub fn sdiv(ctx: &mut EVMContext, contract: &Contract) {
    let (a, neg_a) = sign_u256(ctx.stack.pop());
    let (b, neg_b) = sign_u256(ctx.stack.pop());

    let min = (U256::one() << 255) - U256::one();
    ctx.stack.push(if b.is_zero() {
        U256::zero()
    } else if a == min && b == !U256::one() {
        min
    } else {
        set_sign(a / b, neg_a ^ neg_b)
    })
}

pub fn r#mod(ctx: &mut EVMContext, contract: &Contract) {
    let a = ctx.stack.pop();
    let b = ctx.stack.pop();

    ctx.stack
        .push(if b.is_zero() { U256::zero() } else { a % b })
}

pub fn smod(ctx: &mut EVMContext, contract: &Contract) {
    let (a, neg_a) = sign_u256(ctx.stack.pop());
    let (b, neg_b) = sign_u256(ctx.stack.pop());

    ctx.stack.push(if b.is_zero() {
        U256::zero()
    } else {
        set_sign(a % b, neg_a)
    })
}

pub fn add_mod(ctx: &mut EVMContext, contract: &Contract) {
    let a = ctx.stack.pop();
    let b = ctx.stack.pop();
    let c = ctx.stack.pop();

    ctx.stack.push(if c.is_zero() {
        U256::zero()
    } else {
        (a + b) % c
    })
}

pub fn mul_mod(ctx: &mut EVMContext, contract: &Contract) {
    let a = ctx.stack.pop();
    let b = ctx.stack.pop();
    let c = ctx.stack.pop();

    ctx.stack.push(if c.is_zero() {
        U256::zero()
    } else {
        (a * b) % c
    })
}

pub fn exp(ctx: &mut EVMContext, contract: &Contract) {
    let a = ctx.stack.pop();
    let b = ctx.stack.pop();

    ctx.stack.push(a.pow(b))
}

pub fn sign_extend(ctx: &mut EVMContext, contract: &Contract) {
    let back = ctx.stack.pop();
    if back < U256::from(32) {
        let bit_position = (back.as_u64() * 8 + 7) as usize;
        let num = ctx.stack.pop();

        let bit = num.bit(bit_position);
        let mask = (U256::one() << bit_position) - U256::one();
        ctx.stack.push(if bit { num | !mask } else { num & mask })
    }
}

/// 10s: Comparison & Bitwise Logic Operations
pub fn lt(ctx: &mut EVMContext, contract: &Contract) {
    let a = ctx.stack.pop();
    let b = ctx.stack.pop();

    ctx.stack.push(bool_to_u256(a < b))
}

pub fn gt(ctx: &mut EVMContext, contract: &Contract) {
    let a = ctx.stack.pop();
    let b = ctx.stack.pop();

    ctx.stack.push(bool_to_u256(a > b))
}

pub fn slt(ctx: &mut EVMContext, contract: &Contract) {
    let (a, neg_a) = sign_u256(ctx.stack.pop());
    let (b, neg_b) = sign_u256(ctx.stack.pop());

    let is_positive_lt = a < b && !(neg_a | neg_b);
    let is_negative_lt = a > b && (neg_a & neg_b);
    let has_different_signs = neg_a && !neg_b;

    ctx.stack.push(bool_to_u256(
        is_positive_lt | is_negative_lt | has_different_signs,
    ));
}

pub fn sgt(ctx: &mut EVMContext, contract: &Contract) {
    let (a, neg_a) = sign_u256(ctx.stack.pop());
    let (b, neg_b) = sign_u256(ctx.stack.pop());

    let is_positive_gt = a > b && !(neg_a | neg_b);
    let is_negative_gt = a < b && (neg_a & neg_b);
    let has_different_signs = !neg_a && neg_b;

    ctx.stack.push(bool_to_u256(
        is_positive_gt | is_negative_gt | has_different_signs,
    ));
}

pub fn eq(ctx: &mut EVMContext, contract: &Contract) {
    let a = ctx.stack.pop();
    let b = ctx.stack.pop();

    ctx.stack.push(bool_to_u256(a == b))
}

pub fn is_zero(ctx: &mut EVMContext, contract: &Contract) {
    let a = ctx.stack.pop();

    ctx.stack.push(bool_to_u256(a.is_zero()))
}

pub fn and(ctx: &mut EVMContext, contract: &Contract) {
    let a = ctx.stack.pop();
    let b = ctx.stack.pop();

    ctx.stack.push(a & b)
}

pub fn or(ctx: &mut EVMContext, contract: &Contract) {
    let a = ctx.stack.pop();
    let b = ctx.stack.pop();

    ctx.stack.push(a | b)
}

pub fn xor(ctx: &mut EVMContext, contract: &Contract) {
    let a = ctx.stack.pop();
    let b = ctx.stack.pop();

    ctx.stack.push(a ^ b)
}

pub fn not(ctx: &mut EVMContext, contract: &Contract) {
    let a = ctx.stack.pop();

    ctx.stack.push(!a)
}

pub fn byte(ctx: &mut EVMContext, contract: &Contract) {
    let th = ctx.stack.pop();
    let val = ctx.stack.pop();

    ctx.stack.push(if th > U256::from(32) {
        U256::zero()
    } else {
        (val >> (8 * (31 - th.as_u64() as usize))) & U256::from(0xff)
    })
}

/// 20s: SHA3
pub fn sha3(ctx: &mut EVMContext, contract: &Contract) {
    let offset = ctx.stack.pop();
    let size = ctx.stack.pop();

    let val = ctx
        .memory
        .get(offset.as_u64() as usize, size.as_u64() as usize);

    let hash = Sha3Hasher::digest(val);
    ctx.stack.push(U256::from(hash))
}

/// 30s: Environmental Information
pub fn address(ctx: &mut EVMContext, contract: &Contract) {
    let data = address_to_u256(contract.contract_address);
    ctx.stack.push(data)
}

pub fn balance(ctx: &mut EVMContext, contract: &Contract) {
    let address = u256_to_address(&ctx.stack.pop());
    let balance = ctx.state_db.get_balance(address);
    ctx.stack.push(balance)
}

pub fn origin(ctx: &mut EVMContext, contract: &Contract) {
    ctx.stack.push(address_to_u256(ctx.info.origin))
}

pub fn caller(ctx: &mut EVMContext, contract: &Contract) {
    let data = address_to_u256(contract.caller_address);
    ctx.stack.push(data)
}

pub fn call_value(ctx: &mut EVMContext, contract: &Contract) {
    ctx.stack.push(contract.value)
}

pub fn call_data_load(ctx: &mut EVMContext, contract: &Contract) {
    let input = &contract.input;
    let start = ctx.stack.pop();
    let len = U256::from(32);

    let data = copy_data(input.as_slice(), start, len);
    ctx.stack.push(U256::from(data.as_slice()))
}

pub fn call_data_size(ctx: &mut EVMContext, contract: &Contract) {
    let len = contract.input.len();
    ctx.stack.push(U256::from(len))
}

pub fn call_data_copy(ctx: &mut EVMContext, contract: &Contract) {
    let mem_offset = ctx.stack.pop();
    let data_offset = ctx.stack.pop();
    let len = ctx.stack.pop();

    let data = copy_data(contract.input.as_slice(), data_offset, len);
    ctx.memory.set(mem_offset.as_usize(), data.as_slice())
}

pub fn code_size(ctx: &mut EVMContext, contract: &Contract) {
    let size = U256::from(contract.code.len());
    ctx.stack.push(size)
}

pub fn code_copy(ctx: &mut EVMContext, contract: &Contract) {
    let mem_offset = ctx.stack.pop();
    let code_offset = ctx.stack.pop();
    let len = ctx.stack.pop();

    let val = copy_data(contract.code.as_slice(), code_offset, len);
    ctx.memory.set(mem_offset.as_usize(), val.as_slice())
}

pub fn quota_price(ctx: &mut EVMContext, contract: &Contract) {
    ctx.stack.push(ctx.info.quota_price)
}

pub fn ext_code_size(ctx: &mut EVMContext, contract: &Contract) {
    let address = u256_to_address(&ctx.stack.pop());
    let size = ctx.state_db.get_code_size(address);
    ctx.stack.push(U256::from(size))
}

pub fn ext_code_copy(ctx: &mut EVMContext, contract: &Contract) {
    let address = u256_to_address(&ctx.stack.pop());
    let mem_offset = ctx.stack.pop();
    let code_offset = ctx.stack.pop();
    let len = ctx.stack.pop();
    let code = ctx.state_db.get_code(address);

    let val = copy_data(code.as_slice(), code_offset, len);
    ctx.memory.set(mem_offset.as_usize(), val.as_slice())
}

pub fn return_data_size(ctx: &mut EVMContext, contract: &Contract) {
    ctx.stack.push(U256::from(ctx.return_data.len()))
}

#[inline]
fn bool_to_u256(val: bool) -> U256 {
    if val {
        U256::one()
    } else {
        U256::zero()
    }
}

#[inline]
fn sign_u256(value: U256) -> (U256, bool) {
    let U256(arr) = value;
    let sign = arr[3].leading_zeros() == 0;
    (set_sign(value, sign), sign)
}

#[inline]
fn set_sign(value: U256, sign: bool) -> U256 {
    if sign {
        (!U256::zero() ^ value).overflowing_add(U256::one()).0
    } else {
        value
    }
}

#[inline]
fn address_to_u256(address: Address) -> U256 {
    U256::from(&*H256::from(address))
}

#[inline]
fn u256_to_address(value: &U256) -> Address {
    Address::from(H256::from(value))
}

#[inline]
fn copy_data(source: &[u8], start: U256, size: U256) -> Vec<u8> {
    let source_len = U256::from(source.len());
    let s = u256_min(start, source_len);
    let e = u256_min(s + size, source_len);

    let data = &source[s.as_usize()..e.as_usize()];
    let mut container: Vec<u8> = vec![0; size.as_usize()];
    container[s.as_usize()..e.as_usize()].copy_from_slice(data);
    container
}

#[inline]
fn u256_min(x: U256, y: U256) -> U256 {
    if x > y {
        y
    } else {
        x
    }
}
