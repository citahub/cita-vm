use std::cmp;

use ethereum_types::{H256, U256, U512};

use crate::evm::common;
use crate::evm::ext::DataProvider;
use crate::evm::memory::Memory;
use crate::evm::stack::Stack;
use crate::evm::Error;
use crate::evm::OpCode;
use crate::{Context, InterpreterParams, InterpreterResult, Log};

#[derive(Clone, Debug)]
pub struct InterpreterConf {
    pub eip1283: bool,
    pub stack_limit: u64,
    pub max_create_code_size: u64, // See: https://github.com/ethereum/EIPs/issues/659
    pub max_call_depth: u64,

    pub gas_tier_step: [u64; 8],
    pub gas_exp: u64,                       // Partial payment for an EXP operation.
    pub gas_exp_byte: u64, // Partial payment when multiplied by dlog256(exponent)e for the EXP operation
    pub gas_sha3: u64,     // Paid for each SHA3 operation.
    pub gas_sha3_word: u64, // Paid for each word (rounded up) for input data to a SHA3 operation.
    pub gas_balance: u64,  // Amount of gas to pay for a BALANCE operation.
    pub gas_memory: u64,   // Paid for every additional word when expanding memory.
    pub gas_sload: u64,    //  Paid for a SLOAD operation.
    pub gas_sstore_noop: u64, // eip1283
    pub gas_sstore_init: u64, // Paid for an SSTORE operation when the storage value is set to non-zero from zero.
    pub gas_sstore_clear_refund: u64, // Refund given (added into refund counter) when the storage value is set to zero from non-zero.
    pub gas_sstore_clean: u64, // Paid for an SSTORE operation when the storage value’s zeroness remains unchanged or is set to zero.
    pub gas_sstore_dirty: u64, // eip1283
    pub gas_sstore_reset_clear_refund: u64, // eip1283
    pub gas_sstore_reset_refund: u64, // eip1283
    pub gas_sstore_set: u64,   // eip1283
    pub gas_sstore_clear: u64, // eip1283
    pub gas_sstore_reset: u64, // eip1283
    pub gas_sstore_refund: u64, // eip1283
    pub gas_log: u64,          // Partial payment for a LOG operation.
    pub gas_log_data: u64,     // Paid for each byte in a LOG operation’s data.
    pub gas_log_topic: u64,    // Paid for each topic of a LOG operation.
    pub gas_create: u64,       // Paid for a CREATE operation
    pub gas_jumpdest: u64,     // Paid for a JUMPDEST operation
    pub gas_copy: u64,         // Partial payment for *COPY operations, multiplied by words copied, rounded up.
    pub gas_call: u64,         // Paid for a CALL operation.
    pub gas_call_value_transfer: u64, // Paid for a non-zero value transfer as part of the CALL operation.
    pub gas_call_stipend: u64, // A stipend for the called contract subtracted from Gcallvalue for a non-zero value transfer
    pub gas_self_destruct: u64, // Amount of gas to pay for a SELFDESTRUCT operation.
    pub gas_self_destruct_refund: u64, // Refund given (added into refund counter) for self-destructing an account.
    pub gas_extcode: u64,      // Amount of gas to pay for operations of the set Wextcode.
    pub gas_call_new_account: u64, // Paid for a CALL operation which creates an account
    pub gas_self_destruct_new_account: u64, // Paid for a SELFDESTRUCT operation which creates an account
    pub gas_ext_code_hash: u64, // Piad for a EXTCODEHASH operation. http://eips.ethereum.org/EIPS/eip-1052
}

impl Default for InterpreterConf {
    // The default is version "2d0661f 2018-11-08" of https://ethereum.github.io/yellowpaper/paper.pdf
    // But in order to pass the test, Some modifications must needed.
    //
    // If you want to step through the steps, let the
    fn default() -> Self {
        InterpreterConf {
            eip1283: false,
            stack_limit: 1024,
            max_create_code_size: 24567,
            max_call_depth: 1024,

            gas_tier_step: [0, 2, 3, 5, 8, 10, 20, 0],
            gas_exp: 10,
            gas_exp_byte: 50,
            gas_sha3: 30,
            gas_sha3_word: 6,
            gas_balance: 400,
            gas_memory: 3,
            gas_sload: 200,
            gas_sstore_noop: 200,
            gas_sstore_init: 20000,
            gas_sstore_clear_refund: 15000,
            gas_sstore_clean: 5000,
            gas_sstore_dirty: 200,
            gas_sstore_reset_clear_refund: 19800,
            gas_sstore_reset_refund: 4800,
            gas_sstore_set: 20000,
            gas_sstore_clear: 5000,
            gas_sstore_reset: 5000,
            gas_sstore_refund: 15000,
            gas_log: 375,
            gas_log_data: 8,
            gas_log_topic: 375,
            gas_create: 32000,
            gas_jumpdest: 1,
            gas_copy: 3,
            gas_call: 700,
            gas_call_value_transfer: 9000,
            gas_call_stipend: 2300,
            gas_self_destruct: 5000,
            gas_self_destruct_refund: 24000,
            gas_extcode: 700,
            gas_call_new_account: 25000,
            gas_self_destruct_new_account: 25000,
            gas_ext_code_hash: 400,
        }
    }
}

pub struct Interpreter {
    pub context: Context,
    pub cfg: InterpreterConf,
    pub data_provider: Box<dyn DataProvider>,
    pub params: InterpreterParams,

    gas: u64,
    stack: Stack<U256>,
    mem: Memory,
    logs: Vec<Log>,
    return_data: Vec<u8>,
    mem_gas: u64,
    gas_tmp: u64,
}

impl Interpreter {
    pub fn new(
        context: Context,
        cfg: InterpreterConf,
        data_provider: Box<dyn DataProvider>,
        params: InterpreterParams,
    ) -> Self {
        let gas = params.gas_limit;
        Interpreter {
            context,
            cfg,
            data_provider,
            params,
            gas,
            stack: Stack::with_capacity(1024),
            mem: Memory::default(),
            logs: Vec::new(),
            return_data: Vec::new(),
            mem_gas: 0,
            gas_tmp: 0,
        }
    }

    #[allow(clippy::cognitive_complexity)]
    pub fn run(&mut self) -> Result<InterpreterResult, Error> {
        let mut pc = 0;
        while let Some(op) = self.get_op(pc)? {
            pc += 1;
            // Trace the execution informations
            self.trace(&op, pc);
            // Ensure stack
            let stack_require = op.stack_require();
            if !self.stack.require(stack_require as usize) {
                return Err(Error::OutOfStack);
            }
            if self.stack.len() as u64 - op.stack_require() + op.stack_returns() > self.cfg.stack_limit {
                return Err(Error::OutOfStack);
            }
            // Ensure no state changes in static call
            if self.params.read_only && op.state_changes() {
                return Err(Error::MutableCallInStaticContext);
            }
            // Gas cost and mem expand.
            let op_gas = self.cfg.gas_tier_step[op.gas_price_tier().idx()];
            self.use_gas(op_gas)?;
            match op {
                OpCode::EXP => {
                    let expon = self.stack.back(1);
                    let bytes = ((expon.bits() + 7) / 8) as u64;
                    let gas = self.cfg.gas_exp + self.cfg.gas_exp_byte * bytes;
                    self.use_gas(gas)?;
                }
                OpCode::SHA3 => {
                    let mem_offset = self.stack.back(0);
                    let mem_len = self.stack.back(1);
                    self.mem_gas_work(mem_offset, mem_len)?;
                    self.use_gas(self.cfg.gas_sha3)?;
                    self.use_gas(common::to_word_size(mem_len.low_u64()) * self.cfg.gas_sha3_word)?;
                }
                OpCode::BALANCE => {
                    self.use_gas(self.cfg.gas_balance)?;
                }
                OpCode::CALLDATACOPY => {
                    let mem_offset = self.stack.back(0);
                    let mem_len = self.stack.back(2);
                    self.mem_gas_work(mem_offset, mem_len)?;
                    let gas = common::to_word_size(mem_len.low_u64()) * self.cfg.gas_copy;
                    self.use_gas(gas)?;
                }
                OpCode::CODECOPY => {
                    let mem_offset = self.stack.back(0);
                    let mem_len = self.stack.back(2);
                    self.mem_gas_work(mem_offset, mem_len)?;
                    let gas = common::to_word_size(mem_len.low_u64()) * self.cfg.gas_copy;
                    self.use_gas(gas)?;
                }
                OpCode::EXTCODESIZE => {
                    self.use_gas(self.cfg.gas_extcode)?;
                }
                OpCode::EXTCODECOPY => {
                    let mem_offset = self.stack.back(1);
                    let mem_len = self.stack.back(3);
                    self.use_gas(self.cfg.gas_extcode)?;
                    self.mem_gas_work(mem_offset, mem_len)?;
                    let gas = common::to_word_size(mem_len.low_u64()) * self.cfg.gas_copy;
                    self.use_gas(gas)?;
                }
                OpCode::EXTCODEHASH => {
                    self.use_gas(self.cfg.gas_ext_code_hash)?;
                }
                OpCode::RETURNDATACOPY => {
                    let mem_offset = self.stack.back(0);
                    let size = self.stack.back(2);
                    self.mem_gas_work(mem_offset, size)?;
                    let size_min = cmp::min(self.return_data.len() as u64, size.low_u64());
                    let gas = common::to_word_size(size_min) * self.cfg.gas_copy;
                    self.use_gas(gas)?;
                }
                OpCode::MLOAD => {
                    let mem_offset = self.stack.back(0);
                    let mem_len = U256::from(32);
                    self.mem_gas_work(mem_offset, mem_len)?;
                }
                OpCode::MSTORE => {
                    let mem_offset = self.stack.back(0);
                    let mem_len = U256::from(32);
                    self.mem_gas_work(mem_offset, mem_len)?;
                }
                OpCode::MSTORE8 => {
                    let mem_offset = self.stack.back(0);
                    self.mem_gas_work(mem_offset, U256::one())?;
                }
                OpCode::SLOAD => {
                    self.use_gas(self.cfg.gas_sload)?;
                }
                OpCode::SSTORE => {
                    let address = H256::from(&self.stack.back(0));
                    let current_value = U256::from(&*self.data_provider.get_storage(&self.params.address, &address));
                    let new_value = self.stack.back(1);
                    let original_value =
                        U256::from(&*self.data_provider.get_storage_origin(&self.params.address, &address));
                    let gas: u64 = {
                        if self.cfg.eip1283 {
                            // See https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1283.md
                            if current_value == new_value {
                                self.cfg.gas_sstore_noop
                            } else if original_value == current_value {
                                if original_value.is_zero() {
                                    self.cfg.gas_sstore_init
                                } else {
                                    if new_value.is_zero() {
                                        self.data_provider
                                            .add_refund(&self.params.origin, self.cfg.gas_sstore_clear_refund);
                                    }
                                    self.cfg.gas_sstore_clean
                                }
                            } else {
                                if !original_value.is_zero() {
                                    if current_value.is_zero() {
                                        self.data_provider
                                            .sub_refund(&self.params.origin, self.cfg.gas_sstore_clear_refund);
                                    } else if new_value.is_zero() {
                                        self.data_provider
                                            .add_refund(&self.params.origin, self.cfg.gas_sstore_clear_refund);
                                    }
                                }
                                if original_value == new_value {
                                    if original_value.is_zero() {
                                        self.data_provider
                                            .add_refund(&self.params.origin, self.cfg.gas_sstore_reset_clear_refund);
                                    } else {
                                        self.data_provider
                                            .add_refund(&self.params.origin, self.cfg.gas_sstore_reset_refund);
                                    }
                                }
                                self.cfg.gas_sstore_dirty
                            }
                        } else if current_value.is_zero() && !new_value.is_zero() {
                            self.cfg.gas_sstore_set
                        } else if !current_value.is_zero() && new_value.is_zero() {
                            self.data_provider
                                .add_refund(&self.params.origin, self.cfg.gas_sstore_refund);
                            self.cfg.gas_sstore_clear
                        } else {
                            self.cfg.gas_sstore_reset
                        }
                    };
                    self.use_gas(gas)?;
                }
                OpCode::JUMPDEST => {
                    self.use_gas(self.cfg.gas_jumpdest)?;
                }
                OpCode::LOG0 | OpCode::LOG1 | OpCode::LOG2 | OpCode::LOG3 | OpCode::LOG4 => {
                    let n = op.clone() as u8 - OpCode::LOG0 as u8;
                    let mem_offset = self.stack.back(0);
                    let mem_len = self.stack.back(1);
                    self.mem_gas_work(mem_offset, mem_len)?;
                    let gas = self.cfg.gas_log
                        + self.cfg.gas_log_topic * u64::from(n)
                        + self.cfg.gas_log_data * mem_len.low_u64();
                    self.use_gas(gas)?;
                }
                OpCode::CREATE => {
                    let mem_offset = self.stack.back(1);
                    let mem_len = self.stack.back(2);
                    if mem_len > U256::from(self.cfg.max_create_code_size) {
                        return Err(Error::ExccedMaxCodeSize);
                    }
                    self.mem_gas_work(mem_offset, mem_len)?;
                    self.use_gas(self.cfg.gas_create)?;
                    self.gas_tmp = self.gas - self.gas / 64;
                    self.use_gas(self.gas_tmp)?;
                }
                OpCode::CALL | OpCode::CALLCODE => {
                    let gas_req = self.stack.back(0);
                    let address = common::u256_to_address(&self.stack.back(1));
                    let value = self.stack.back(2);
                    let mem_offset = self.stack.back(3);
                    let mem_len = self.stack.back(4);
                    let out_offset = self.stack.back(5);
                    let out_len = self.stack.back(6);

                    self.use_gas(self.cfg.gas_call)?;

                    let is_value_transfer = !value.is_zero();
                    if op == OpCode::CALL && is_value_transfer && self.data_provider.is_empty(&address) {
                        self.use_gas(self.cfg.gas_call_new_account)?;
                    }
                    if is_value_transfer {
                        self.use_gas(self.cfg.gas_call_value_transfer)?;
                    }
                    self.mem_gas_work(mem_offset, mem_len)?;
                    self.mem_gas_work(out_offset, out_len)?;
                    self.gas_tmp = cmp::min(self.gas - self.gas / 64, gas_req.low_u64());
                    self.use_gas(self.gas_tmp)?;
                }
                OpCode::RETURN => {
                    let mem_offset = self.stack.back(0);
                    let mem_len = self.stack.back(1);
                    self.mem_gas_work(mem_offset, mem_len)?;
                }
                OpCode::DELEGATECALL | OpCode::STATICCALL => {
                    let gas_req = self.stack.back(0);
                    let mem_offset = self.stack.back(2);
                    let mem_len = self.stack.back(3);
                    let out_offset = self.stack.back(4);
                    let out_len = self.stack.back(5);
                    self.use_gas(self.cfg.gas_call)?;
                    self.mem_gas_work(mem_offset, mem_len)?;
                    self.mem_gas_work(out_offset, out_len)?;
                    self.gas_tmp = cmp::min(self.gas - self.gas / 64, gas_req.low_u64());
                    self.use_gas(self.gas_tmp)?;
                }
                OpCode::CREATE2 => {
                    let mem_offset = self.stack.back(1);
                    let mem_len = self.stack.back(2);
                    if mem_len > U256::from(self.cfg.max_create_code_size) {
                        return Err(Error::ExccedMaxCodeSize);
                    }
                    self.mem_gas_work(mem_offset, mem_len)?;
                    self.use_gas(self.cfg.gas_create)?;
                    self.use_gas(common::to_word_size(mem_len.low_u64()) * self.cfg.gas_sha3_word)?;
                    self.gas_tmp = self.gas - self.gas / 64;
                    self.use_gas(self.gas_tmp)?;
                }
                OpCode::REVERT => {
                    let mem_offset = self.stack.back(0);
                    let mem_len = self.stack.back(1);
                    self.mem_gas_work(mem_offset, mem_len)?;
                }
                OpCode::SELFDESTRUCT => {
                    let address = self.stack.peek();
                    self.use_gas(self.cfg.gas_self_destruct)?;
                    if !self.data_provider.get_balance(&self.params.address).is_zero()
                        && self.data_provider.is_empty(&common::u256_to_address(&address))
                    {
                        self.use_gas(self.cfg.gas_self_destruct_new_account)?;
                    }
                    self.data_provider
                        .add_refund(&self.params.origin, self.cfg.gas_self_destruct_refund);
                }
                _ => {}
            }
            // Step 5: Let's dance!
            match op {
                OpCode::STOP => break,
                OpCode::ADD => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(a.overflowing_add(b).0);
                }
                OpCode::MUL => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(a.overflowing_mul(b).0);
                }
                OpCode::SUB => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(a.overflowing_sub(b).0);
                }
                OpCode::DIV => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(if b.is_zero() { U256::zero() } else { a / b })
                }
                OpCode::SDIV => {
                    let (a, neg_a) = common::get_sign(self.stack.pop());
                    let (b, neg_b) = common::get_sign(self.stack.pop());
                    let min = (U256::one() << 255) - U256::one();
                    self.stack.push(if b.is_zero() {
                        U256::zero()
                    } else if a == min && b == !U256::one() {
                        min
                    } else {
                        common::set_sign(a / b, neg_a ^ neg_b)
                    })
                }
                OpCode::MOD => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(if !b.is_zero() { a % b } else { U256::zero() });
                }
                OpCode::SMOD => {
                    let ua = self.stack.pop();
                    let ub = self.stack.pop();
                    let (a, sign_a) = common::get_sign(ua);
                    let b = common::get_sign(ub).0;
                    self.stack.push(if !b.is_zero() {
                        let c = a % b;
                        common::set_sign(c, sign_a)
                    } else {
                        U256::zero()
                    });
                }
                OpCode::ADDMOD => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    let c = self.stack.pop();
                    self.stack.push(if !c.is_zero() {
                        let res = U512::from(a);
                        let res = res.overflowing_add(U512::from(b)).0;
                        let res = res % U512::from(c);
                        U256::from(res)
                    } else {
                        U256::zero()
                    });
                }
                OpCode::MULMOD => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    let c = self.stack.pop();
                    self.stack.push(if !c.is_zero() {
                        let res = U512::from(a);
                        let res = res.overflowing_mul(U512::from(b)).0;
                        let res = res % U512::from(c);
                        U256::from(res)
                    } else {
                        U256::zero()
                    });
                }
                OpCode::EXP => {
                    let base = self.stack.pop();
                    let expon = self.stack.pop();
                    let res = base.overflowing_pow(expon).0;
                    self.stack.push(res);
                }
                OpCode::SIGNEXTEND => {
                    let bit = self.stack.pop();
                    if bit < U256::from(32) {
                        let number = self.stack.pop();
                        let bit_position = (bit.low_u64() * 8 + 7) as usize;
                        let bit = number.bit(bit_position);
                        let mask = (U256::one() << bit_position) - U256::one();
                        self.stack.push(if bit { number | !mask } else { number & mask });
                    }
                }
                OpCode::LT => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(common::bool_to_u256(a < b));
                }
                OpCode::GT => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(common::bool_to_u256(a > b));
                }
                OpCode::SLT => {
                    let (a, neg_a) = common::get_sign(self.stack.pop());
                    let (b, neg_b) = common::get_sign(self.stack.pop());
                    let is_positive_lt = a < b && !(neg_a | neg_b);
                    let is_negative_lt = a > b && (neg_a & neg_b);
                    let has_different_signs = neg_a && !neg_b;
                    self.stack.push(common::bool_to_u256(
                        is_positive_lt | is_negative_lt | has_different_signs,
                    ));
                }
                OpCode::SGT => {
                    let (a, neg_a) = common::get_sign(self.stack.pop());
                    let (b, neg_b) = common::get_sign(self.stack.pop());
                    let is_positive_gt = a > b && !(neg_a | neg_b);
                    let is_negative_gt = a < b && (neg_a & neg_b);
                    let has_different_signs = !neg_a && neg_b;
                    self.stack.push(common::bool_to_u256(
                        is_positive_gt | is_negative_gt | has_different_signs,
                    ));
                }
                OpCode::EQ => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(common::bool_to_u256(a == b));
                }
                OpCode::ISZERO => {
                    let a = self.stack.pop();
                    self.stack.push(common::bool_to_u256(a.is_zero()));
                }
                OpCode::AND => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(a & b);
                }
                OpCode::OR => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(a | b);
                }
                OpCode::XOR => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(a ^ b);
                }
                OpCode::NOT => {
                    let a = self.stack.pop();
                    self.stack.push(!a);
                }
                OpCode::BYTE => {
                    let word = self.stack.pop();
                    let val = self.stack.pop();
                    let byte = if word < U256::from(32) {
                        (val >> (8 * (31 - word.low_u64() as usize))) & U256::from(0xff)
                    } else {
                        U256::zero()
                    };
                    self.stack.push(byte);
                }
                OpCode::SHL => {
                    const CONST_256: U256 = U256([256, 0, 0, 0]);
                    let shift = self.stack.pop();
                    let value = self.stack.pop();
                    let result = if shift >= CONST_256 {
                        U256::zero()
                    } else {
                        value << (shift.as_u32() as usize)
                    };
                    self.stack.push(result);
                }
                OpCode::SHR => {
                    const CONST_256: U256 = U256([256, 0, 0, 0]);
                    let shift = self.stack.pop();
                    let value = self.stack.pop();
                    let result = if shift >= CONST_256 {
                        U256::zero()
                    } else {
                        value >> (shift.as_u32() as usize)
                    };
                    self.stack.push(result);
                }
                OpCode::SAR => {
                    const CONST_256: U256 = U256([256, 0, 0, 0]);
                    const CONST_HIBIT: U256 = U256([0, 0, 0, 0x8000_0000_0000_0000]);
                    let shift = self.stack.pop();
                    let value = self.stack.pop();
                    let sign = value & CONST_HIBIT != U256::zero();
                    let result = if shift >= CONST_256 {
                        if sign {
                            U256::max_value()
                        } else {
                            U256::zero()
                        }
                    } else {
                        let shift = shift.as_u32() as usize;
                        let mut shifted = value >> shift;
                        if sign {
                            shifted = shifted | (U256::max_value() << (256 - shift));
                        }
                        shifted
                    };
                    self.stack.push(result);
                }
                OpCode::SHA3 => {
                    let mem_offset = self.stack.pop();
                    let mem_len = self.stack.pop();
                    let k = self
                        .data_provider
                        .sha3(self.mem.get(mem_offset.low_u64() as usize, mem_len.low_u64() as usize));
                    self.stack.push(U256::from(k));
                }
                OpCode::ADDRESS => {
                    self.stack.push(common::address_to_u256(self.params.address));
                }
                OpCode::BALANCE => {
                    let address = common::u256_to_address(&self.stack.pop());
                    let balance = self.data_provider.get_balance(&address);
                    self.stack.push(balance);
                }
                OpCode::ORIGIN => {
                    self.stack.push(common::address_to_u256(self.params.origin));
                }
                OpCode::CALLER => {
                    self.stack.push(common::address_to_u256(self.params.sender));
                }
                OpCode::CALLVALUE => {
                    self.stack.push(self.params.value);
                }
                OpCode::CALLDATALOAD => {
                    let big_id = self.stack.pop();
                    let id = big_id.low_u64() as usize;
                    let max = id.wrapping_add(32);
                    if !self.params.input.is_empty() {
                        let bound = cmp::min(self.params.input.len(), max);
                        if id < bound && big_id < U256::from(self.params.input.len()) {
                            let mut v = [0u8; 32];
                            v[0..bound - id].clone_from_slice(&self.params.input[id..bound]);
                            self.stack.push(U256::from(&v[..]))
                        } else {
                            self.stack.push(U256::zero())
                        }
                    } else {
                        self.stack.push(U256::zero())
                    }
                }
                OpCode::CALLDATASIZE => {
                    self.stack.push(U256::from(self.params.input.len()));
                }
                OpCode::CALLDATACOPY => {
                    let mem_offset = self.stack.pop();
                    let raw_offset = self.stack.pop();
                    let size = self.stack.pop();

                    let data = common::copy_data(self.params.input.as_slice(), raw_offset, size);
                    self.mem.set(mem_offset.as_usize(), data.as_slice());
                }
                OpCode::CODESIZE => {
                    self.stack.push(U256::from(self.params.contract.code_data.len()));
                }
                OpCode::CODECOPY => {
                    let mem_offset = self.stack.pop();
                    let raw_offset = self.stack.pop();
                    let size = self.stack.pop();
                    let data = common::copy_data(self.params.contract.code_data.as_slice(), raw_offset, size);
                    self.mem.set(mem_offset.as_usize(), data.as_slice());
                }
                OpCode::GASPRICE => {
                    self.stack.push(self.params.gas_price);
                }
                OpCode::EXTCODESIZE => {
                    let address = common::u256_to_address(&self.stack.pop());
                    let len = self.data_provider.get_code_size(&address);
                    self.stack.push(U256::from(len));
                }
                OpCode::EXTCODECOPY => {
                    let address = common::u256_to_address(&self.stack.pop());
                    let mem_offset = self.stack.pop();
                    let code_offset = self.stack.pop();
                    let size = self.stack.pop();
                    let code = self.data_provider.get_code(&address);
                    let data = common::copy_data(code.as_slice(), code_offset, size);
                    self.mem.set(mem_offset.as_usize(), data.as_slice())
                }
                OpCode::RETURNDATASIZE => self.stack.push(U256::from(self.return_data.len())),
                OpCode::RETURNDATACOPY => {
                    let mem_offset = self.stack.pop();
                    let raw_offset = self.stack.pop();
                    let size = self.stack.pop();
                    let return_data_len = U256::from(self.return_data.len());
                    if raw_offset.saturating_add(size) > return_data_len {
                        return Err(Error::OutOfBounds);
                    }
                    let data = common::copy_data(&self.return_data, raw_offset, size);
                    self.mem.set(mem_offset.as_usize(), data.as_slice())
                }
                OpCode::EXTCODEHASH => {
                    let address = common::u256_to_address(&self.stack.pop());
                    let hash = self.data_provider.get_code_hash(&address);
                    self.stack.push(U256::from(hash));
                }
                OpCode::BLOCKHASH => {
                    let block_number = self.stack.pop();
                    let block_hash = self.data_provider.get_block_hash(&block_number);
                    self.stack.push(U256::from(&*block_hash));
                }
                OpCode::COINBASE => {
                    self.stack.push(common::address_to_u256(self.context.coinbase));
                }
                OpCode::TIMESTAMP => {
                    self.stack.push(U256::from(self.context.timestamp));
                }
                OpCode::NUMBER => {
                    self.stack.push(self.context.number);
                }
                OpCode::DIFFICULTY => {
                    self.stack.push(self.context.difficulty);
                }
                OpCode::GASLIMIT => {
                    // Get the block's gas limit
                    self.stack.push(U256::from(self.context.gas_limit));
                }
                OpCode::POP => {
                    self.stack.pop();
                }
                OpCode::MLOAD => {
                    let offset = self.stack.pop().as_u64();
                    let word = self.mem.get(offset as usize, 32);
                    self.stack.push(U256::from(word));
                }
                OpCode::MSTORE => {
                    let offset = self.stack.pop();
                    let word = self.stack.pop();
                    let word = &<[u8; 32]>::from(word)[..];
                    self.mem.set(offset.low_u64() as usize, word);
                }
                OpCode::MSTORE8 => {
                    let offset = self.stack.pop();
                    let word = self.stack.pop();
                    self.mem.set(offset.low_u64() as usize, &[word.low_u64() as u8]);
                }
                OpCode::SLOAD => {
                    let key = H256::from(self.stack.pop());
                    let word = U256::from(&*self.data_provider.get_storage(&self.params.address, &key));
                    self.stack.push(word);
                }
                OpCode::SSTORE => {
                    let address = H256::from(&self.stack.pop());
                    let value = self.stack.pop();
                    self.data_provider
                        .set_storage(&self.params.address, address, H256::from(&value));
                }
                OpCode::JUMP => {
                    let jump = self.stack.pop();
                    self.pre_jump(jump)?;
                    pc = jump.low_u64();
                }
                OpCode::JUMPI => {
                    let jump = self.stack.pop();
                    let condition = self.stack.pop();
                    if !condition.is_zero() {
                        self.pre_jump(jump)?;
                        pc = jump.low_u64();
                    }
                }
                OpCode::PC => {
                    self.stack.push(U256::from(pc - 1));
                }
                OpCode::MSIZE => {
                    self.stack.push(U256::from(self.mem.len()));
                }
                OpCode::GAS => {
                    self.stack.push(U256::from(self.gas));
                }
                OpCode::JUMPDEST => {}
                OpCode::PUSH1
                | OpCode::PUSH2
                | OpCode::PUSH3
                | OpCode::PUSH4
                | OpCode::PUSH5
                | OpCode::PUSH6
                | OpCode::PUSH7
                | OpCode::PUSH8
                | OpCode::PUSH9
                | OpCode::PUSH10
                | OpCode::PUSH11
                | OpCode::PUSH12
                | OpCode::PUSH13
                | OpCode::PUSH14
                | OpCode::PUSH15
                | OpCode::PUSH16
                | OpCode::PUSH17
                | OpCode::PUSH18
                | OpCode::PUSH19
                | OpCode::PUSH20
                | OpCode::PUSH21
                | OpCode::PUSH22
                | OpCode::PUSH23
                | OpCode::PUSH24
                | OpCode::PUSH25
                | OpCode::PUSH26
                | OpCode::PUSH27
                | OpCode::PUSH28
                | OpCode::PUSH29
                | OpCode::PUSH30
                | OpCode::PUSH31
                | OpCode::PUSH32 => {
                    let n = op as u8 - OpCode::PUSH1 as u8 + 1;
                    let e = pc + u64::from(n);
                    let e = cmp::min(e, self.params.contract.code_data.len() as u64);
                    let r = U256::from(&self.params.contract.code_data[pc as usize..e as usize]);
                    pc = e;
                    self.stack.push(r);
                }
                OpCode::DUP1
                | OpCode::DUP2
                | OpCode::DUP3
                | OpCode::DUP4
                | OpCode::DUP5
                | OpCode::DUP6
                | OpCode::DUP7
                | OpCode::DUP8
                | OpCode::DUP9
                | OpCode::DUP10
                | OpCode::DUP11
                | OpCode::DUP12
                | OpCode::DUP13
                | OpCode::DUP14
                | OpCode::DUP15
                | OpCode::DUP16 => {
                    let p = op as u8 - OpCode::DUP1 as u8;
                    self.stack.dup(p as usize);
                }
                OpCode::SWAP1
                | OpCode::SWAP2
                | OpCode::SWAP3
                | OpCode::SWAP4
                | OpCode::SWAP5
                | OpCode::SWAP6
                | OpCode::SWAP7
                | OpCode::SWAP8
                | OpCode::SWAP9
                | OpCode::SWAP10
                | OpCode::SWAP11
                | OpCode::SWAP12
                | OpCode::SWAP13
                | OpCode::SWAP14
                | OpCode::SWAP15
                | OpCode::SWAP16 => {
                    let p = op as u8 - OpCode::SWAP1 as u8 + 1;
                    self.stack.swap(p as usize);
                }
                OpCode::LOG0 | OpCode::LOG1 | OpCode::LOG2 | OpCode::LOG3 | OpCode::LOG4 => {
                    let n = op as u8 - OpCode::LOG0 as u8;
                    let mem_offset = self.stack.pop();
                    let mem_len = self.stack.pop();
                    let mut topics: Vec<H256> = Vec::new();
                    for _ in 0..n {
                        let r = H256::from(self.stack.pop());
                        topics.push(r);
                    }
                    let data = self.mem.get(mem_offset.low_u64() as usize, mem_len.low_u64() as usize);
                    self.logs.push(Log(self.params.address, topics, Vec::from(data)));
                }
                OpCode::CREATE | OpCode::CREATE2 => {
                    // Clear return data buffer before creating new call frame.
                    self.return_data = vec![];

                    let value = self.stack.pop();
                    let mem_offset = self.stack.pop();
                    let mem_len = self.stack.pop();
                    let salt = H256::from({
                        match op {
                            OpCode::CREATE => U256::zero(),
                            OpCode::CREATE2 => self.stack.pop(),
                            _ => panic!("instruction can only be CREATE/CREATE2 checked above"),
                        }
                    });
                    let data = self.mem.get(mem_offset.low_u64() as usize, mem_len.low_u64() as usize);
                    // Exit immediately if value > balance.
                    if value > self.data_provider.get_balance(&self.params.address) {
                        self.gas += self.gas_tmp;
                        self.stack.push(U256::zero());
                        continue;
                    }
                    // Exit immediately if depth exceed limit.
                    if self.params.depth >= self.cfg.max_call_depth {
                        self.gas += self.gas_tmp;
                        self.stack.push(U256::zero());
                        continue;
                    }
                    let mut params = InterpreterParams::default();
                    params.origin = self.params.origin;
                    params.sender = self.params.address;
                    params.gas_limit = self.gas_tmp;
                    params.gas_price = self.params.gas_price;
                    params.input = Vec::from(data);
                    params.value = value;
                    params.extra = salt;
                    params.depth = self.params.depth + 1;
                    let r = self.data_provider.call(op, params);
                    match r {
                        Ok(data) => match data {
                            InterpreterResult::Create(_, gas, logs, add) => {
                                self.stack.push(common::address_to_u256(add));
                                self.gas += gas;
                                self.logs.extend(logs);
                            }
                            InterpreterResult::Revert(ret, gas) => {
                                self.stack.push(U256::zero());
                                self.gas += gas;
                                self.return_data = ret;
                            }
                            _ => {}
                        },
                        Err(_) => {
                            self.stack.push(U256::zero());
                        }
                    }
                }
                OpCode::CALL | OpCode::CALLCODE | OpCode::DELEGATECALL | OpCode::STATICCALL => {
                    let _ = self.stack.pop();
                    let address = common::u256_to_address(&self.stack.pop());
                    let value = {
                        if op == OpCode::CALL || op == OpCode::CALLCODE {
                            self.stack.pop()
                        } else {
                            U256::zero()
                        }
                    };
                    let mem_offset = self.stack.pop();
                    let mem_len = self.stack.pop();
                    let out_offset = self.stack.pop();
                    let out_len = self.stack.pop();

                    if op == OpCode::CALL && self.params.read_only && !value.is_zero() {
                        return Err(Error::MutableCallInStaticContext);
                    }

                    // Clear return data buffer before creating new call frame.
                    self.return_data = vec![];

                    let mut gas = self.gas_tmp;
                    // Add stipend (only when CALL|CALLCODE and value > 0)
                    if !value.is_zero() {
                        gas += self.cfg.gas_call_stipend;
                    }
                    // Exit immediately if value > balance.
                    if value > self.data_provider.get_balance(&self.params.address) {
                        self.gas += gas;
                        self.stack.push(U256::zero());
                        continue;
                    }
                    // Exit immediately if depth exceed limit.
                    if self.params.depth >= self.cfg.max_call_depth {
                        self.gas += gas;
                        self.stack.push(U256::zero());
                        continue;
                    }
                    let data = self.mem.get(mem_offset.low_u64() as usize, mem_len.low_u64() as usize);
                    let mut params = InterpreterParams::default();
                    params.origin = self.params.origin;
                    params.gas_limit = gas;
                    params.gas_price = self.params.gas_price;
                    params.contract.code_address = address;
                    params.contract.code_data = self.data_provider.get_code(&params.contract.code_address).to_vec();
                    params.input = Vec::from(data);
                    params.depth = self.params.depth + 1;
                    // The flag `read_only` should geneticed from the parent.
                    // Let's explain it. Take a example belows:
                    //
                    // User -> ContractA(STATICCALL)-> ContractB(CALL) -> ContractC(Set a Log)
                    //         read_only=false         read_only=true     read_only=???
                    //
                    // What's the value of `read_only` in ContractC?
                    // Fine, the answer should be `true`. So, ContractC can't do any changes.
                    params.read_only = self.params.read_only;

                    match op {
                        OpCode::CALL => {
                            params.sender = self.params.address;
                            params.receiver = address;
                            params.address = address;
                            params.value = value;
                        }
                        OpCode::CALLCODE => {
                            params.sender = self.params.address;
                            params.receiver = self.params.address;
                            params.address = self.params.address;
                            params.value = value;
                        }
                        OpCode::DELEGATECALL => {
                            params.sender = self.params.sender;
                            params.receiver = self.params.address;
                            params.address = self.params.address;
                            // DELEGATECALL should NEVER transfer balances, we set the
                            // params.value to the CALLVALUE opcode.
                            //
                            // Take below as an example:
                            //
                            // User -> value=10 ->  Contract A -> value=0 -> Contract B (op CALLVALUE)
                            //
                            // Contract A call a DELEGATECALL to Contract B, with no balance, but
                            // when CALLVALUE did in Contract B, value=10 should be given.
                            params.value = self.params.value;
                            params.disable_transfer_value = true;
                        }
                        OpCode::STATICCALL => {
                            params.sender = self.params.address;
                            params.receiver = address;
                            params.address = address;
                            params.read_only = true;
                        }
                        _ => {}
                    }
                    let r = self.data_provider.call(op, params);
                    match r {
                        Ok(data) => match data {
                            InterpreterResult::Normal(mut ret, gas, logs) => {
                                self.stack.push(U256::one());
                                self.return_data = ret.clone();
                                if ret.len() > out_len.low_u64() as usize {
                                    ret.resize(out_len.low_u64() as usize, 0u8);
                                }
                                self.mem.set(out_offset.low_u64() as usize, ret.as_slice());
                                self.gas += gas;
                                self.logs.extend(logs);
                            }
                            InterpreterResult::Revert(mut ret, gas) => {
                                self.stack.push(U256::zero());
                                self.return_data = ret.clone();
                                if ret.len() > out_len.low_u64() as usize {
                                    ret.resize(out_len.low_u64() as usize, 0u8);
                                }
                                self.mem.set(out_offset.low_u64() as usize, ret.as_slice());
                                self.gas += gas;
                            }
                            _ => {}
                        },
                        Err(_) => {
                            self.stack.push(U256::zero());
                        }
                    }
                }
                OpCode::RETURN => {
                    let mem_offset = self.stack.pop();
                    let mem_len = self.stack.pop();
                    let r = self.mem.get(mem_offset.low_u64() as usize, mem_len.low_u64() as usize);
                    let return_data = Vec::from(r);
                    return Ok(InterpreterResult::Normal(
                        return_data.clone(),
                        self.gas,
                        self.logs.clone(),
                    ));
                }
                OpCode::REVERT => {
                    let mem_offset = self.stack.pop();
                    let mem_len = self.stack.pop();
                    let r = self.mem.get(mem_offset.low_u64() as usize, mem_len.low_u64() as usize);
                    let return_data = Vec::from(r);
                    return Ok(InterpreterResult::Revert(return_data.clone(), self.gas));
                }
                OpCode::SELFDESTRUCT => {
                    let address = self.stack.pop();
                    let b = self
                        .data_provider
                        .selfdestruct(&self.params.address, &common::u256_to_address(&address));
                    if !b {
                        // Imaging this, what if we `SELFDESTRUCT` a contract twice,
                        // what will happend?
                        //
                        // Obviously, we should not `add_refund` for each `SELFDESTRUCT`.
                        // But it is difficult to know whether the address has been destructed,
                        // so an eclectic approach is sub the refund after call.
                        self.data_provider
                            .sub_refund(&self.params.origin, self.cfg.gas_self_destruct_refund);
                    }
                    break;
                }
            }
            log::debug!("");
        }
        Ok(InterpreterResult::Normal(vec![], self.gas, self.logs.clone()))
    }

    fn use_gas(&mut self, gas: u64) -> Result<(), Error> {
        log::debug!("[Gas] - {}", gas);
        if self.gas < gas {
            return Err(Error::OutOfGas);
        }
        self.gas -= gas;
        Ok(())
    }

    fn mem_gas_cost(&mut self, size: u64) -> u64 {
        let goc = common::mem_gas_cost(size, self.cfg.gas_memory);
        if goc > self.mem_gas {
            let fee = goc - self.mem_gas;
            self.mem_gas = goc;
            fee
        } else {
            0
        }
    }

    fn mem_gas_work(&mut self, mem_offset: U256, mem_len: U256) -> Result<(), Error> {
        if mem_len.is_zero() {
            return Ok(());
        }
        let (mem_sum, b) = mem_offset.overflowing_add(mem_len);
        // Ethereum's max block gas is 8000000, when mem size > 1024 * 64,
        // about 8000000 gas should be fired. That's means this transaction
        // will never success.
        //
        // But to keep some imagination, 1G is used as the ceil.
        if b || mem_sum.bits() > 64 || mem_sum.low_u64() > 1024 * 1024 * 1024 {
            return Err(Error::OutOfGas);
        }
        if mem_len != U256::zero() {
            let gas = self.mem_gas_cost(mem_sum.low_u64());
            self.use_gas(gas)?;
        }
        self.mem.expand(mem_sum.low_u64() as usize);
        Ok(())
    }

    fn get_byte(&self, n: u64) -> Option<u8> {
        if n < self.params.contract.code_data.len() as u64 {
            return Some(self.params.contract.code_data[n as usize]);
        }
        None
    }

    fn get_op(&self, n: u64) -> Result<Option<OpCode>, Error> {
        match self.get_byte(n) {
            Some(a) => match OpCode::from_u8(a) {
                Some(b) => Ok(Some(b)),
                None => Err(Error::InvalidOpcode),
            },
            None => Ok(None),
        }
    }

    fn pre_jump(&self, n: U256) -> Result<(), Error> {
        if n.bits() > 63 {
            return Err(Error::InvalidJumpDestination);
        }
        let n = n.low_u64() as usize;
        if n >= self.params.contract.code_data.len() {
            return Err(Error::InvalidJumpDestination);
        }
        // Only JUMPDESTs allowed for destinations
        if self.params.contract.code_data[n] != OpCode::JUMPDEST as u8 {
            return Err(Error::InvalidJumpDestination);
        }
        Ok(())
    }

    /// Function trace outputs execution informations.
    fn trace(&self, op: &OpCode, pc: u64) {
        if *op >= OpCode::PUSH1 && *op <= OpCode::PUSH32 {
            let n = op.clone() as u8 - OpCode::PUSH1 as u8 + 1;
            let r = {
                if pc + u64::from(n) > self.params.contract.code_data.len() as u64 {
                    U256::zero()
                } else {
                    U256::from(&self.params.contract.code_data[pc as usize..(pc + u64::from(n)) as usize])
                }
            };
            log::debug!("[OP] {} {:#x} gas={}", op, r, self.gas);
        } else {
            log::debug!("[OP] {} gas={}", op, self.gas);
        }
        log::debug!("[STACK]");
        let l = self.stack.data().len();
        for i in 0..l {
            log::debug!("[{}] {:#x}", i, self.stack.back(i));
        }
        log::debug!("[MEM] len={}", self.mem.len());
    }
}

#[cfg(test)]
mod tests {
    // The unit tests just carried from go-ethereum.
    use ethereum_types::Address;

    use super::super::extmock;
    use super::*;

    fn default_interpreter() -> Interpreter {
        let mut it = Interpreter::new(
            Context::default(),
            InterpreterConf::default(),
            Box::new(extmock::DataProviderMock::default()),
            InterpreterParams::default(),
        );
        it.context.gas_limit = 1_000_000;
        it.params.gas_limit = 1_000_000;
        it.gas = 1_000_000;
        it
    }

    #[test]
    fn test_op_byte() {
        let data = vec![
            (
                "ABCDEF0908070605040302010000000000000000000000000000000000000000",
                "0",
                "AB",
            ),
            (
                "ABCDEF0908070605040302010000000000000000000000000000000000000000",
                "1",
                "CD",
            ),
            (
                "00CDEF090807060504030201ffffffffffffffffffffffffffffffffffffffff",
                "0",
                "00",
            ),
            (
                "ABCDEF0908070605040302010000000000000000000000000000000000000000",
                "1",
                "CD",
            ),
            (
                "00CDEF090807060504030201ffffffffffffffffffffffffffffffffffffffff",
                "0",
                "00",
            ),
            (
                "00CDEF090807060504030201ffffffffffffffffffffffffffffffffffffffff",
                "1",
                "CD",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000102030",
                "31",
                "30",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000102030",
                "30",
                "20",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "32",
                "00",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "4294967295",
                "00",
            ),
        ];
        for (val, th, expected) in data {
            let mut it = default_interpreter();
            it.stack
                .push_n(&[U256::from(val), U256::from(th.parse::<u64>().unwrap())]);
            it.params.contract.code_data = vec![OpCode::BYTE as u8];
            it.run().unwrap();
            assert_eq!(it.stack.pop(), U256::from(expected));
        }
    }

    #[test]
    fn test_op_shl() {
        let data = vec![
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "00",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "01",
                "0000000000000000000000000000000000000000000000000000000000000002",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "ff",
                "8000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0100",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0101",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "00",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "01",
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "ff",
                "8000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0100",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000000",
                "01",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "01",
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe",
            ),
        ];
        for (x, y, expected) in data {
            let mut it = default_interpreter();
            it.stack.push_n(&[U256::from(x), U256::from(y)]);
            it.params.contract.code_data = vec![OpCode::SHL as u8];
            it.run().unwrap();
            assert_eq!(it.stack.pop(), U256::from(expected));
        }
    }

    #[test]
    fn test_op_shr() {
        let data = vec![
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "00",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "01",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000000",
                "01",
                "4000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000000",
                "ff",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000000",
                "0100",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000000",
                "0101",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "00",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "01",
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "ff",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0100",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000000",
                "01",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
        ];
        for (x, y, expected) in data {
            let mut it = default_interpreter();
            it.stack.push_n(&[U256::from(x), U256::from(y)]);
            it.params.contract.code_data = vec![OpCode::SHR as u8];
            it.run().unwrap();
            assert_eq!(it.stack.pop(), U256::from(expected));
        }
    }

    #[test]
    fn test_op_sar() {
        let data = vec![
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "00",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "01",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000000",
                "01",
                "c000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000000",
                "ff",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000000",
                "0100",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000000",
                "0101",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "00",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "01",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "ff",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0100",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000000",
                "01",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "4000000000000000000000000000000000000000000000000000000000000000",
                "fe",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "f8",
                "000000000000000000000000000000000000000000000000000000000000007f",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "fe",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "ff",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0100",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
        ];
        for (x, y, expected) in data {
            let mut it = default_interpreter();
            it.stack.push_n(&[U256::from(x), U256::from(y)]);
            it.params.contract.code_data = vec![OpCode::SAR as u8];
            it.run().unwrap();
            assert_eq!(it.stack.pop(), U256::from(expected));
        }
    }

    #[test]
    fn test_op_sgt() {
        let data = vec![
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000001",
                "8000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000001",
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "8000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffb",
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd",
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffb",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
        ];
        for (x, y, expected) in data {
            let mut it = default_interpreter();
            it.stack.push_n(&[U256::from(x), U256::from(y)]);
            it.params.contract.code_data = vec![OpCode::SGT as u8];
            it.run().unwrap();
            assert_eq!(it.stack.pop(), U256::from(expected));
        }
    }

    #[test]
    fn test_op_slt() {
        let data = vec![
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000001",
                "8000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000001",
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "8000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffb",
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd",
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffb",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
        ];
        for (x, y, expected) in data {
            let mut it = default_interpreter();
            it.stack.push_n(&[U256::from(x), U256::from(y)]);
            it.params.contract.code_data = vec![OpCode::SLT as u8];
            it.run().unwrap();
            assert_eq!(it.stack.pop(), U256::from(expected));
        }
    }

    #[test]
    fn test_op_mstore() {
        let mut it = default_interpreter();
        let v = "abcdef00000000000000abba000000000deaf000000c0de00100000000133700";
        it.stack.push_n(&[U256::from(v), U256::zero()]);
        it.params.contract.code_data = vec![OpCode::MSTORE as u8];
        it.run().unwrap();
        assert_eq!(it.mem.get(0, 32), hex::decode(v).unwrap().as_slice());
        it.stack.push_n(&[U256::one(), U256::zero()]);
        it.params.contract.code_data = vec![OpCode::MSTORE as u8];
        it.run().unwrap();
        assert_eq!(
            it.mem.get(0, 32),
            hex::decode("0000000000000000000000000000000000000000000000000000000000000001")
                .unwrap()
                .as_slice()
        );
    }

    #[test]
    fn test_op_sstore_eip_1283() {
        // From https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1283.md#test-cases
        let data = vec![
            ("60006000556000600055", 412, 0, 0),
            ("60006000556001600055", 20212, 0, 0),
            ("60016000556000600055", 20212, 19800, 0),
            ("60016000556002600055", 20212, 0, 0),
            ("60016000556001600055", 20212, 0, 0),
            ("60006000556000600055", 5212, 15000, 1),
            ("60006000556001600055", 5212, 4800, 1),
            ("60006000556002600055", 5212, 0, 1),
            ("60026000556000600055", 5212, 15000, 1),
            ("60026000556003600055", 5212, 0, 1),
            ("60026000556001600055", 5212, 4800, 1),
            ("60026000556002600055", 5212, 0, 1),
            ("60016000556000600055", 5212, 15000, 1),
            ("60016000556002600055", 5212, 0, 1),
            ("60016000556001600055", 412, 0, 1),
            ("600160005560006000556001600055", 40218, 19800, 0),
            ("600060005560016000556000600055", 10218, 19800, 1),
        ];
        for (code, use_gas, refund, origin) in data {
            let mut it = default_interpreter();
            it.cfg.eip1283 = true;
            assert_eq!(it.gas, it.context.gas_limit);
            it.data_provider
                .set_storage_origin(&it.params.contract.code_address, H256::zero(), H256::from(origin));
            it.data_provider
                .set_storage(&it.params.contract.code_address, H256::zero(), H256::from(origin));
            it.params.contract.code_data = hex::decode(code).unwrap();
            it.run().unwrap();
            assert_eq!(it.gas, it.context.gas_limit - use_gas);
            assert_eq!(it.data_provider.get_refund(&Address::zero()), refund);
        }
    }

    #[test]
    fn test_op_invalid() {
        let mut it = default_interpreter();
        it.params.contract.code_data = hex::decode("fb").unwrap();
        let r = it.run();
        assert!(r.is_err());
        assert_eq!(r.err(), Some(Error::InvalidOpcode))
    }
}
