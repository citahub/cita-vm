use std::cmp;

use ethereum_types::{Address, H256, U256, U512};
use log::debug;

use crate::evm::common;
use crate::evm::err;
use crate::evm::ext;
use crate::evm::memory;
use crate::evm::opcodes;
use crate::evm::stack;

#[derive(Clone, Debug, Default)]
pub struct Context {
    pub gas_limit: u64,
    pub coinbase: Address,
    pub number: U256,
    pub timestamp: u64,
    pub difficulty: U256,
}

// Log is the data struct for LOG0...LOG4.
// The members are "Address: Address, Topics: Vec<H256>, Body: Vec<u8>"
#[derive(Clone, Debug)]
pub struct Log(pub Address, pub Vec<H256>, pub Vec<u8>);

#[derive(Clone, Debug)]
pub enum InterpreterResult {
    // Return data, remain gas, logs.
    Normal(Vec<u8>, u64, Vec<Log>),
    // Return data, remain gas
    Revert(Vec<u8>, u64),
    // Return data, remain gas, logs, contract address
    Create(Vec<u8>, u64, Vec<Log>, Address),
}

#[derive(Clone,Debug)]
pub struct InterpreterConf {
    pub no_empty: bool,
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
            no_empty: false,
            eip1283: false,
            stack_limit: 1024,
            max_create_code_size: std::u64::MAX,
            max_call_depth: 1024,

            gas_tier_step: [0, 2, 3, 5, 8, 10, 20, 0],
            gas_exp: 10,
            gas_exp_byte: 10,//50,
            gas_sha3: 30,
            gas_sha3_word: 6,
            gas_balance: 20, //400,
            gas_memory: 3,
            gas_sload: 50, //200,
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
            gas_call: 40,//700,
            gas_call_value_transfer: 9000,
            gas_call_stipend: 2300,
            gas_self_destruct: 0,//5000,
            gas_self_destruct_refund: 24000,
            gas_extcode: 20,//700,
            gas_call_new_account: 25000,
            gas_self_destruct_new_account: 0,//25000,
            gas_ext_code_hash: 400,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Contract {
    pub code_address: Address,
    pub code_data: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct InterpreterParams {
    pub origin: Address,   // Who send the transaction
    pub sender: Address,   // Who send the call
    pub receiver: Address, // Who receive the transaction or call
    pub address: Address,  // Which storage used

    pub value: U256,
    pub input: Vec<u8>,
    pub nonce: U256,
    pub gas_limit: u64,
    pub gas_price: U256,

    pub read_only: bool,
    pub contract: Contract,
    pub extra: H256,
    pub is_create: bool,
    pub disable_transfer_value: bool,
    pub depth: u64,
}

pub struct Interpreter {
    pub context: Context,
    pub cfg: InterpreterConf,
    pub data_provider: Box<ext::DataProvider>,
    pub params: InterpreterParams,

    gas: u64,
    stack: stack::Stack<U256>,
    mem: memory::Memory,
    logs: Vec<Log>,
    return_data: Vec<u8>,
    mem_gas: u64,
    gas_tmp: u64,
}

impl Interpreter {
    pub fn new(
        context: Context,
        cfg: InterpreterConf,
        data_provider: Box<ext::DataProvider>,
        params: InterpreterParams,
    ) -> Self {
        let gas = params.gas_limit;
        Interpreter {
            context,
            cfg,
            data_provider,
            params,
            gas,
            stack: stack::Stack::with_capacity(1024),
            mem: memory::Memory::default(),
            logs: Vec::new(),
            return_data: Vec::new(),
            mem_gas: 0,
            gas_tmp: 0,
        }
    }

    #[allow(clippy::cognitive_complexity)]
    pub fn run(&mut self) -> Result<InterpreterResult, err::Error> {
        let mut pc = 0;
        while let Some(op) = self.get_op(pc)? {
            pc += 1;
            // Trace the execution informations
            self.trace(&op, pc);
            // Ensure stack
            let stack_require = op.stack_require();
            if !self.stack.require(stack_require as usize) {
                return Err(err::Error::OutOfStack);
            }
            if self.stack.len() as u64 - op.stack_require() + op.stack_returns() > self.cfg.stack_limit {
                return Err(err::Error::OutOfStack);
            }
            // Ensure no state changes in static call
            if self.params.read_only && op.state_changes() {
                return Err(err::Error::MutableCallInStaticContext);
            }
            // Gas cost and mem expand.
            let op_gas = self.cfg.gas_tier_step[op.gas_price_tier().idx()];
            self.use_gas(op_gas)?;
            //println!("********** interpert run op {:?}",op);
            match op {
                opcodes::OpCode::EXP => {
                    let expon = self.stack.back(1);
                    let bytes = ((expon.bits() + 7) / 8) as u64;
                    let gas = self.cfg.gas_exp + self.cfg.gas_exp_byte * bytes;
                    self.use_gas(gas)?;
                }
                opcodes::OpCode::SHA3 => {
                    let mem_offset = self.stack.back(0);
                    let mem_len = self.stack.back(1);
                    self.mem_gas_work(mem_offset, mem_len)?;
                    self.use_gas(self.cfg.gas_sha3)?;
                    self.use_gas(common::to_word_size(mem_len.low_u64()) * self.cfg.gas_sha3_word)?;
                }
                opcodes::OpCode::BALANCE => {
                    self.use_gas(self.cfg.gas_balance)?;
                }
                opcodes::OpCode::CALLDATACOPY => {
                    let mem_offset = self.stack.back(0);
                    let mem_len = self.stack.back(2);
                    self.mem_gas_work(mem_offset, mem_len)?;
                    let gas = common::to_word_size(mem_len.low_u64()) * self.cfg.gas_copy;
                    self.use_gas(gas)?;
                }
                opcodes::OpCode::CODECOPY => {
                    let mem_offset = self.stack.back(0);
                    let mem_len = self.stack.back(2);
                    self.mem_gas_work(mem_offset, mem_len)?;
                    let gas = common::to_word_size(mem_len.low_u64()) * self.cfg.gas_copy;
                    self.use_gas(gas)?;
                }
                opcodes::OpCode::EXTCODESIZE => {
                    self.use_gas(self.cfg.gas_extcode)?;
                }
                opcodes::OpCode::EXTCODECOPY => {
                    let mem_offset = self.stack.back(1);
                    let mem_len = self.stack.back(3);
                    self.use_gas(self.cfg.gas_extcode)?;
                    self.mem_gas_work(mem_offset, mem_len)?;
                    let gas = common::to_word_size(mem_len.low_u64()) * self.cfg.gas_copy;
                    self.use_gas(gas)?;
                }
                opcodes::OpCode::EXTCODEHASH => {
                    self.use_gas(self.cfg.gas_ext_code_hash)?;
                }
                opcodes::OpCode::RETURNDATACOPY => {
                    let mem_offset = self.stack.back(0);
                    let size = self.stack.back(2);
                    self.mem_gas_work(mem_offset, size)?;
                    //println!("***** RETURNDATACOPY return data {:?},size {:?}",self.return_data,size);
                    let size_min = cmp::min(self.return_data.len() as u64, size.low_u64());
                    let gas = common::to_word_size(size_min) * self.cfg.gas_copy;
                    self.use_gas(gas)?;
                }
                opcodes::OpCode::MLOAD => {
                    let mem_offset = self.stack.back(0);
                    let mem_len = U256::from(32);
                    self.mem_gas_work(mem_offset, mem_len)?;
                }
                opcodes::OpCode::MSTORE => {
                    let mem_offset = self.stack.back(0);
                    let mem_len = U256::from(32);
                    self.mem_gas_work(mem_offset, mem_len)?;
                }
                opcodes::OpCode::MSTORE8 => {
                    let mem_offset = self.stack.back(0);
                    self.mem_gas_work(mem_offset, U256::one())?;
                }
                opcodes::OpCode::SLOAD => {
                    self.use_gas(self.cfg.gas_sload)?;
                }
                opcodes::OpCode::SSTORE => {
                    let address = H256::from(&self.stack.back(0));
                    let current_value = U256::from(&*self.data_provider.get_storage(&self.params.address, &address));
                    let new_value = self.stack.back(1);
                    let original_value =
                        U256::from(&*self.data_provider.get_storage_origin(&self.params.address, &address));

                    //println!("************ SSTORE address {:?} cur {:?} new {:?}",address,current_value,new_value);
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
                opcodes::OpCode::JUMPDEST => {
                    self.use_gas(self.cfg.gas_jumpdest)?;
                }
                opcodes::OpCode::LOG0
                | opcodes::OpCode::LOG1
                | opcodes::OpCode::LOG2
                | opcodes::OpCode::LOG3
                | opcodes::OpCode::LOG4 => {
                    let n = op.clone() as u8 - opcodes::OpCode::LOG0 as u8;
                    let mem_offset = self.stack.back(0);
                    let mem_len = self.stack.back(1);
                    self.mem_gas_work(mem_offset, mem_len)?;
                    let gas = self.cfg.gas_log
                        + self.cfg.gas_log_topic * u64::from(n)
                        + self.cfg.gas_log_data * mem_len.low_u64();
                    self.use_gas(gas)?;
                }
                opcodes::OpCode::CREATE => {
                    let mem_offset = self.stack.back(1);
                    let mem_len = self.stack.back(2);
                    //println!("******* mem len {:?} max code size {:?}",mem_len,U256::from(self.cfg.max_create_code_size));
                    if mem_len > U256::from(self.cfg.max_create_code_size) {
                        return Err(err::Error::ExccedMaxCodeSize);
                    }
                    self.mem_gas_work(mem_offset, mem_len)?;
                    self.use_gas(self.cfg.gas_create)?;
                    self.gas_tmp = self.gas - self.gas / 64;
                    self.use_gas(self.gas_tmp)?;
                }
                opcodes::OpCode::CALL | opcodes::OpCode::CALLCODE => {
                    let gas_req = self.stack.back(0);
                    let address = common::u256_to_address(&self.stack.back(1));
                    let value = self.stack.back(2);
                    let mem_offset = self.stack.back(3);
                    let mem_len = self.stack.back(4);
                    let out_offset = self.stack.back(5);
                    let out_len = self.stack.back(6);

                    self.use_gas(self.cfg.gas_call)?;

                    let is_value_transfer = !value.is_zero();

                    //println!("\n****** value transfer {:?} \n",self.data_provider.get_balance(&address));
                    if op == opcodes::OpCode::CALL
                        && (!self.cfg.no_empty && !self.data_provider.exist(&address) || (self.cfg.no_empty && is_value_transfer && self.data_provider.is_empty(&address))) {
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
                opcodes::OpCode::RETURN => {
                    let mem_offset = self.stack.back(0);
                    let mem_len = self.stack.back(1);
                    self.mem_gas_work(mem_offset, mem_len)?;
                }
                opcodes::OpCode::DELEGATECALL | opcodes::OpCode::STATICCALL => {
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
                opcodes::OpCode::CREATE2 => {
                    let mem_offset = self.stack.back(1);
                    let mem_len = self.stack.back(2);
                    //println!("******* mem len 2 {:?} max code size {:?}",mem_len,U256::from(self.cfg.max_create_code_size));
                    if mem_len > U256::from(self.cfg.max_create_code_size) {
                        return Err(err::Error::ExccedMaxCodeSize);
                    }
                    self.mem_gas_work(mem_offset, mem_len)?;
                    self.use_gas(self.cfg.gas_create)?;
                    self.use_gas(common::to_word_size(mem_len.low_u64()) * self.cfg.gas_sha3_word)?;
                    self.gas_tmp = self.gas - self.gas / 64;
                    self.use_gas(self.gas_tmp)?;
                }
                opcodes::OpCode::REVERT => {
                    let mem_offset = self.stack.back(0);
                    let mem_len = self.stack.back(1);
                    self.mem_gas_work(mem_offset, mem_len)?;
                }
                opcodes::OpCode::SELFDESTRUCT => {
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
                opcodes::OpCode::STOP => break,
                opcodes::OpCode::ADD => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(a.overflowing_add(b).0);
                }
                opcodes::OpCode::MUL => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(a.overflowing_mul(b).0);
                }
                opcodes::OpCode::SUB => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(a.overflowing_sub(b).0);
                }
                opcodes::OpCode::DIV => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(if b.is_zero() { U256::zero() } else { a / b })
                }
                opcodes::OpCode::SDIV => {
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
                opcodes::OpCode::MOD => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(if !b.is_zero() { a % b } else { U256::zero() });
                }
                opcodes::OpCode::SMOD => {
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
                opcodes::OpCode::ADDMOD => {
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
                opcodes::OpCode::MULMOD => {
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
                opcodes::OpCode::EXP => {
                    let base = self.stack.pop();
                    let expon = self.stack.pop();
                    let res = base.overflowing_pow(expon).0;
                    self.stack.push(res);
                }
                opcodes::OpCode::SIGNEXTEND => {
                    let bit = self.stack.pop();
                    if bit < U256::from(32) {
                        let number = self.stack.pop();
                        let bit_position = (bit.low_u64() * 8 + 7) as usize;
                        let bit = number.bit(bit_position);
                        let mask = (U256::one() << bit_position) - U256::one();
                        self.stack.push(if bit { number | !mask } else { number & mask });
                    }
                }
                opcodes::OpCode::LT => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(common::bool_to_u256(a < b));
                }
                opcodes::OpCode::GT => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(common::bool_to_u256(a > b));
                }
                opcodes::OpCode::SLT => {
                    let (a, neg_a) = common::get_sign(self.stack.pop());
                    let (b, neg_b) = common::get_sign(self.stack.pop());
                    let is_positive_lt = a < b && !(neg_a | neg_b);
                    let is_negative_lt = a > b && (neg_a & neg_b);
                    let has_different_signs = neg_a && !neg_b;
                    self.stack.push(common::bool_to_u256(
                        is_positive_lt | is_negative_lt | has_different_signs,
                    ));
                }
                opcodes::OpCode::SGT => {
                    let (a, neg_a) = common::get_sign(self.stack.pop());
                    let (b, neg_b) = common::get_sign(self.stack.pop());
                    let is_positive_gt = a > b && !(neg_a | neg_b);
                    let is_negative_gt = a < b && (neg_a & neg_b);
                    let has_different_signs = !neg_a && neg_b;
                    self.stack.push(common::bool_to_u256(
                        is_positive_gt | is_negative_gt | has_different_signs,
                    ));
                }
                opcodes::OpCode::EQ => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(common::bool_to_u256(a == b));
                }
                opcodes::OpCode::ISZERO => {
                    let a = self.stack.pop();
                    self.stack.push(common::bool_to_u256(a.is_zero()));
                }
                opcodes::OpCode::AND => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(a & b);
                }
                opcodes::OpCode::OR => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(a | b);
                }
                opcodes::OpCode::XOR => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(a ^ b);
                }
                opcodes::OpCode::NOT => {
                    let a = self.stack.pop();
                    self.stack.push(!a);
                }
                opcodes::OpCode::BYTE => {
                    let word = self.stack.pop();
                    let val = self.stack.pop();
                    let byte = if word < U256::from(32) {
                        (val >> (8 * (31 - word.low_u64() as usize))) & U256::from(0xff)
                    } else {
                        U256::zero()
                    };
                    self.stack.push(byte);
                }
                opcodes::OpCode::SHL => {
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
                opcodes::OpCode::SHR => {
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
                opcodes::OpCode::SAR => {
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
                opcodes::OpCode::SHA3 => {
                    let mem_offset = self.stack.pop();
                    let mem_len = self.stack.pop();
                    let k = self
                        .data_provider
                        .sha3(self.mem.get(mem_offset.low_u64() as usize, mem_len.low_u64() as usize));
                    self.stack.push(U256::from(k));
                }
                opcodes::OpCode::ADDRESS => {
                    self.stack.push(common::address_to_u256(self.params.address));
                }
                opcodes::OpCode::BALANCE => {
                    let address = common::u256_to_address(&self.stack.pop());
                    let balance = self.data_provider.get_balance(&address);
                    self.stack.push(balance);
                }
                opcodes::OpCode::ORIGIN => {
                    self.stack.push(common::address_to_u256(self.params.origin));
                }
                opcodes::OpCode::CALLER => {
                    self.stack.push(common::address_to_u256(self.params.sender));
                }
                opcodes::OpCode::CALLVALUE => {
                    self.stack.push(self.params.value);
                }
                opcodes::OpCode::CALLDATALOAD => {
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
                opcodes::OpCode::CALLDATASIZE => {
                    self.stack.push(U256::from(self.params.input.len()));
                }
                opcodes::OpCode::CALLDATACOPY => {
                    let mem_offset = self.stack.pop();
                    let raw_offset = self.stack.pop();
                    let size = self.stack.pop();

                    let data = common::copy_data(self.params.input.as_slice(), raw_offset, size);
                    self.mem.set(mem_offset.as_usize(), data.as_slice());
                }
                opcodes::OpCode::CODESIZE => {
                    self.stack.push(U256::from(self.params.contract.code_data.len()));
                }
                opcodes::OpCode::CODECOPY => {
                    let mem_offset = self.stack.pop();
                    let raw_offset = self.stack.pop();
                    let size = self.stack.pop();
                    let data = common::copy_data(self.params.contract.code_data.as_slice(), raw_offset, size);
                    self.mem.set(mem_offset.as_usize(), data.as_slice());
                }
                opcodes::OpCode::GASPRICE => {
                    self.stack.push(self.params.gas_price);
                }
                opcodes::OpCode::EXTCODESIZE => {
                    let address = common::u256_to_address(&self.stack.pop());
                    let len = self.data_provider.get_code_size(&address);
                    self.stack.push(U256::from(len));
                }
                opcodes::OpCode::EXTCODECOPY => {
                    let address = common::u256_to_address(&self.stack.pop());
                    let mem_offset = self.stack.pop();
                    let code_offset = self.stack.pop();
                    let size = self.stack.pop();
                    let code = self.data_provider.get_code(&address);
                    let data = common::copy_data(code.as_slice(), code_offset, size);
                    self.mem.set(mem_offset.as_usize(), data.as_slice())
                }
                opcodes::OpCode::RETURNDATASIZE => self.stack.push(U256::from(self.return_data.len())),
                opcodes::OpCode::RETURNDATACOPY => {
                    let mem_offset = self.stack.pop();
                    let raw_offset = self.stack.pop();
                    let size = self.stack.pop();
                    let return_data_len = U256::from(self.return_data.len());
                    println!("***** offsert {:?} size {:?} return_len {:?} memoff {:?}",raw_offset,size,return_data_len,mem_offset);
                    if raw_offset.saturating_add(size) > return_data_len {
                        return Err(err::Error::OutOfBounds);
                    }
                    let data = common::copy_data(&self.return_data, raw_offset, size);
                    self.mem.set(mem_offset.as_usize(), data.as_slice())
                }
                opcodes::OpCode::EXTCODEHASH => {
                    let address = common::u256_to_address(&self.stack.pop());
                    let hash = self.data_provider.get_code_hash(&address);
                    self.stack.push(U256::from(hash));
                }
                opcodes::OpCode::BLOCKHASH => {
                    let block_number = self.stack.pop();
                    let block_hash = self.data_provider.get_block_hash(&block_number);
                    self.stack.push(U256::from(&*block_hash));
                }
                opcodes::OpCode::COINBASE => {
                    self.stack.push(common::address_to_u256(self.context.coinbase));
                }
                opcodes::OpCode::TIMESTAMP => {
                    self.stack.push(U256::from(self.context.timestamp));
                }
                opcodes::OpCode::NUMBER => {
                    self.stack.push(self.context.number);
                }
                opcodes::OpCode::DIFFICULTY => {
                    self.stack.push(self.context.difficulty);
                }
                opcodes::OpCode::GASLIMIT => {
                    // Get the block's gas limit
                    self.stack.push(U256::from(self.context.gas_limit));
                }
                opcodes::OpCode::POP => {
                    self.stack.pop();
                }
                opcodes::OpCode::MLOAD => {
                    let offset = self.stack.pop().as_u64();
                    let word = self.mem.get(offset as usize, 32);
                    self.stack.push(U256::from(word));
                }
                opcodes::OpCode::MSTORE => {
                    let offset = self.stack.pop();
                    let word = self.stack.pop();
                    let word = &<[u8; 32]>::from(word)[..];
                    self.mem.set(offset.low_u64() as usize, word);
                }
                opcodes::OpCode::MSTORE8 => {
                    let offset = self.stack.pop();
                    let word = self.stack.pop();
                    self.mem.set(offset.low_u64() as usize, &[word.low_u64() as u8]);
                }
                opcodes::OpCode::SLOAD => {
                    let key = H256::from(self.stack.pop());
                    let word = U256::from(&*self.data_provider.get_storage(&self.params.address, &key));
                    self.stack.push(word);
                }
                opcodes::OpCode::SSTORE => {
                    let address = H256::from(&self.stack.pop());
                    let value = self.stack.pop();
                    self.data_provider
                        .set_storage(&self.params.address, address, H256::from(&value));
                }
                opcodes::OpCode::JUMP => {
                    let jump = self.stack.pop();
                    self.pre_jump(jump)?;
                    pc = jump.low_u64();
                }
                opcodes::OpCode::JUMPI => {
                    let jump = self.stack.pop();
                    let condition = self.stack.pop();
                    if !condition.is_zero() {
                        self.pre_jump(jump)?;
                        pc = jump.low_u64();
                    }
                }
                opcodes::OpCode::PC => {
                    self.stack.push(U256::from(pc - 1));
                }
                opcodes::OpCode::MSIZE => {
                    self.stack.push(U256::from(self.mem.len()));
                }
                opcodes::OpCode::GAS => {
                    self.stack.push(U256::from(self.gas));
                }
                opcodes::OpCode::JUMPDEST => {}
                opcodes::OpCode::PUSH1
                | opcodes::OpCode::PUSH2
                | opcodes::OpCode::PUSH3
                | opcodes::OpCode::PUSH4
                | opcodes::OpCode::PUSH5
                | opcodes::OpCode::PUSH6
                | opcodes::OpCode::PUSH7
                | opcodes::OpCode::PUSH8
                | opcodes::OpCode::PUSH9
                | opcodes::OpCode::PUSH10
                | opcodes::OpCode::PUSH11
                | opcodes::OpCode::PUSH12
                | opcodes::OpCode::PUSH13
                | opcodes::OpCode::PUSH14
                | opcodes::OpCode::PUSH15
                | opcodes::OpCode::PUSH16
                | opcodes::OpCode::PUSH17
                | opcodes::OpCode::PUSH18
                | opcodes::OpCode::PUSH19
                | opcodes::OpCode::PUSH20
                | opcodes::OpCode::PUSH21
                | opcodes::OpCode::PUSH22
                | opcodes::OpCode::PUSH23
                | opcodes::OpCode::PUSH24
                | opcodes::OpCode::PUSH25
                | opcodes::OpCode::PUSH26
                | opcodes::OpCode::PUSH27
                | opcodes::OpCode::PUSH28
                | opcodes::OpCode::PUSH29
                | opcodes::OpCode::PUSH30
                | opcodes::OpCode::PUSH31
                | opcodes::OpCode::PUSH32 => {
                    let n = op as u8 - opcodes::OpCode::PUSH1 as u8 + 1;
                    let e = pc + u64::from(n);
                    let e = cmp::min(e, self.params.contract.code_data.len() as u64);
                    let r = U256::from(&self.params.contract.code_data[pc as usize..e as usize]);
                    pc = e;
                    self.stack.push(r);
                }
                opcodes::OpCode::DUP1
                | opcodes::OpCode::DUP2
                | opcodes::OpCode::DUP3
                | opcodes::OpCode::DUP4
                | opcodes::OpCode::DUP5
                | opcodes::OpCode::DUP6
                | opcodes::OpCode::DUP7
                | opcodes::OpCode::DUP8
                | opcodes::OpCode::DUP9
                | opcodes::OpCode::DUP10
                | opcodes::OpCode::DUP11
                | opcodes::OpCode::DUP12
                | opcodes::OpCode::DUP13
                | opcodes::OpCode::DUP14
                | opcodes::OpCode::DUP15
                | opcodes::OpCode::DUP16 => {
                    let p = op as u8 - opcodes::OpCode::DUP1 as u8;
                    self.stack.dup(p as usize);
                }
                opcodes::OpCode::SWAP1
                | opcodes::OpCode::SWAP2
                | opcodes::OpCode::SWAP3
                | opcodes::OpCode::SWAP4
                | opcodes::OpCode::SWAP5
                | opcodes::OpCode::SWAP6
                | opcodes::OpCode::SWAP7
                | opcodes::OpCode::SWAP8
                | opcodes::OpCode::SWAP9
                | opcodes::OpCode::SWAP10
                | opcodes::OpCode::SWAP11
                | opcodes::OpCode::SWAP12
                | opcodes::OpCode::SWAP13
                | opcodes::OpCode::SWAP14
                | opcodes::OpCode::SWAP15
                | opcodes::OpCode::SWAP16 => {
                    let p = op as u8 - opcodes::OpCode::SWAP1 as u8 + 1;
                    self.stack.swap(p as usize);
                }
                opcodes::OpCode::LOG0
                | opcodes::OpCode::LOG1
                | opcodes::OpCode::LOG2
                | opcodes::OpCode::LOG3
                | opcodes::OpCode::LOG4 => {
                    let n = op as u8 - opcodes::OpCode::LOG0 as u8;
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
                opcodes::OpCode::CREATE | opcodes::OpCode::CREATE2 => {
                    // Clear return data buffer before creating new call frame.
                    self.return_data = vec![];

                    let value = self.stack.pop();
                    let mem_offset = self.stack.pop();
                    let mem_len = self.stack.pop();
                    let salt = H256::from({
                        match op {
                            opcodes::OpCode::CREATE => U256::zero(),
                            opcodes::OpCode::CREATE2 => self.stack.pop(),
                            _ => panic!("instruction can only be CREATE/CREATE2 checked above"),
                        }
                    });
                    let data = self.mem.get(mem_offset.low_u64() as usize, mem_len.low_u64() as usize);
                    // Exit immediately if value > balance.
                    //println!("\n********** in create value {:?} address {:?} balance {:?} \n",
                    //         value,self.params.address,self.data_provider.get_balance(&self.params.address));
                    if value > self.data_provider.get_balance(&self.params.address) {
                        self.gas += self.gas_tmp;
                        self.stack.push(U256::zero());
                        //println!("******** unused_gas {:?} add tmp gas {:?}",self.gas,self.gas_tmp);
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
                                //println!("***** InterpreterResult::Revert return data {:?}",self.return_data);

                            }
                            _ => {}
                        },
                        Err(_) => {
                            self.stack.push(U256::zero());
                        }
                    }
                }
                opcodes::OpCode::CALL
                | opcodes::OpCode::CALLCODE
                | opcodes::OpCode::DELEGATECALL
                | opcodes::OpCode::STATICCALL => {
                    let _ = self.stack.pop();
                    let address = common::u256_to_address(&self.stack.pop());
                    let value = {
                        if op == opcodes::OpCode::CALL || op == opcodes::OpCode::CALLCODE {
                            self.stack.pop()
                        } else {
                            U256::zero()
                        }
                    };
                    let mem_offset = self.stack.pop();
                    let mem_len = self.stack.pop();
                    let out_offset = self.stack.pop();
                    let out_len = self.stack.pop();

                    if op == opcodes::OpCode::CALL && self.params.read_only && !value.is_zero() {
                        return Err(err::Error::MutableCallInStaticContext);
                    }

                    // Clear return data buffer before creating new call frame.
                    self.return_data = vec![];

                    let mut gas = self.gas_tmp;
                    // Add stipend (only when CALL|CALLCODE and value > 0)
                    if !value.is_zero() {
                        gas += self.cfg.gas_call_stipend;
                    }
                    // Exit immediately if value > balance.
                    //println!("\n********** in call  value {:?} address {:?} balance {:?} gas {:?} \n",
                    //         value,self.params.address,self.data_provider.get_balance(&self.params.address),gas);

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
                        opcodes::OpCode::CALL => {
                            params.sender = self.params.address;
                            params.receiver = address;
                            params.address = address;
                            params.value = value;
                        }
                        opcodes::OpCode::CALLCODE => {
                            params.sender = self.params.address;
                            params.receiver = self.params.address;
                            params.address = self.params.address;
                            params.value = value;
                        }
                        opcodes::OpCode::DELEGATECALL => {
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
                        opcodes::OpCode::STATICCALL => {
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
                               // println!("***** normal result return data {:?} gas {:?} self.gas {:?}",self.return_data,gas,self.gas);
                            }
                            InterpreterResult::Revert(mut ret, gas) => {
                                self.stack.push(U256::zero());
                                self.return_data = ret.clone();
                                //println!("***** revert result return data {:?}",self.return_data);
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
                opcodes::OpCode::RETURN => {
                    let mem_offset = self.stack.pop();
                    let mem_len = self.stack.pop();
                    let r = self.mem.get(mem_offset.low_u64() as usize, mem_len.low_u64() as usize);
                    let return_data = Vec::from(r);
                    //println!("***** return op return data {:?}",return_data);
                    return Ok(InterpreterResult::Normal(
                        return_data.clone(),
                        self.gas,
                        self.logs.clone(),
                    ));
                }
                opcodes::OpCode::REVERT => {
                    let mem_offset = self.stack.pop();
                    let mem_len = self.stack.pop();
                    let r = self.mem.get(mem_offset.low_u64() as usize, mem_len.low_u64() as usize);
                    let return_data = Vec::from(r);
                    //println!("***** revert op return data {:?}",return_data);
                    return Ok(InterpreterResult::Revert(return_data.clone(), self.gas));
                }
                opcodes::OpCode::SELFDESTRUCT => {
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
            debug!("");
        }
        Ok(InterpreterResult::Normal(vec![], self.gas, self.logs.clone()))
    }

    fn use_gas(&mut self, gas: u64) -> Result<(), err::Error> {
        debug!("[Gas] - {}", gas);
        //println!("******** [use_gas] self {:?} gas {:?}",self.gas, gas);
        if self.gas < gas {
            return Err(err::Error::OutOfGas);
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

    fn mem_gas_work(&mut self, mem_offset: U256, mem_len: U256) -> Result<(), err::Error> {
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
            return Err(err::Error::OutOfGas);
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

    fn get_op(&self, n: u64) -> Result<Option<opcodes::OpCode>, err::Error> {
        match self.get_byte(n) {
            Some(a) => match opcodes::OpCode::from_u8(a) {
                Some(b) => Ok(Some(b)),
                None => Err(err::Error::InvalidOpcode),
            },
            None => Ok(None),
        }
    }

    fn pre_jump(&self, n: U256) -> Result<(), err::Error> {
        if n.bits() > 63 {
            return Err(err::Error::InvalidJumpDestination);
        }
        let n = n.low_u64() as usize;
        if n >= self.params.contract.code_data.len() {
            return Err(err::Error::InvalidJumpDestination);
        }
        // Only JUMPDESTs allowed for destinations
        if self.params.contract.code_data[n] != opcodes::OpCode::JUMPDEST as u8 {
            return Err(err::Error::InvalidJumpDestination);
        }
        Ok(())
    }

    /// Function trace outputs execution informations.
    fn trace(&self, op: &opcodes::OpCode, pc: u64) {
        if *op >= opcodes::OpCode::PUSH1 && *op <= opcodes::OpCode::PUSH32 {
            let n = op.clone() as u8 - opcodes::OpCode::PUSH1 as u8 + 1;
            let r = {
                if pc + u64::from(n) > self.params.contract.code_data.len() as u64 {
                    U256::zero()
                } else {
                    U256::from(&self.params.contract.code_data[pc as usize..(pc + u64::from(n)) as usize])
                }
            };
            debug!("[OP] {} {:#x} gas={}", op, r, self.gas);
        } else {
            debug!("[OP] {} gas={}", op, self.gas);
        }
        debug!("[STACK]");
        let l = self.stack.data().len();
        for i in 0..l {
            debug!("[{}] {:#x}", i, self.stack.back(i));
        }
        debug!("[MEM] len={}", self.mem.len());
    }
}

#[cfg(test)]
mod tests {
    // The unit tests just carried from go-ethereum.
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
            let mut it = default_interpreter();;
            it.stack
                .push_n(&[U256::from(val), U256::from(th.parse::<u64>().unwrap())]);
            it.params.contract.code_data = vec![opcodes::OpCode::BYTE as u8];
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
            let mut it = default_interpreter();;
            it.stack.push_n(&[U256::from(x), U256::from(y)]);
            it.params.contract.code_data = vec![opcodes::OpCode::SHL as u8];
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
            let mut it = default_interpreter();;
            it.stack.push_n(&[U256::from(x), U256::from(y)]);
            it.params.contract.code_data = vec![opcodes::OpCode::SHR as u8];
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
            let mut it = default_interpreter();;
            it.stack.push_n(&[U256::from(x), U256::from(y)]);
            it.params.contract.code_data = vec![opcodes::OpCode::SAR as u8];
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
            let mut it = default_interpreter();;
            it.stack.push_n(&[U256::from(x), U256::from(y)]);
            it.params.contract.code_data = vec![opcodes::OpCode::SGT as u8];
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
            let mut it = default_interpreter();;
            it.stack.push_n(&[U256::from(x), U256::from(y)]);
            it.params.contract.code_data = vec![opcodes::OpCode::SLT as u8];
            it.run().unwrap();
            assert_eq!(it.stack.pop(), U256::from(expected));
        }
    }

    #[test]
    fn test_op_mstore() {
        let mut it = default_interpreter();;
        let v = "abcdef00000000000000abba000000000deaf000000c0de00100000000133700";
        it.stack.push_n(&[U256::from(v), U256::zero()]);
        it.params.contract.code_data = vec![opcodes::OpCode::MSTORE as u8];
        it.run().unwrap();
        assert_eq!(it.mem.get(0, 32), hex::decode(v).unwrap().as_slice());
        it.stack.push_n(&[U256::one(), U256::zero()]);
        it.params.contract.code_data = vec![opcodes::OpCode::MSTORE as u8];
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
        let mut it = default_interpreter();;
        it.params.contract.code_data = hex::decode("fb").unwrap();
        let r = it.run();
        assert!(r.is_err());
        assert_eq!(r.err(), Some(err::Error::InvalidOpcode))
    }
}
