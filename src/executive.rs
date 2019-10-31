use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use cita_trie::DB;
use ethereum_types::{Address, H256, U256};
use hashbrown::{HashMap, HashSet};
use rlp::RlpStream;

use crate::common;
use crate::err::Error;
use crate::evm;
use crate::evm::native;
use crate::riscv;
use crate::state::{self, State, StateObjectInfo};
use crate::{Context, Contract, InterpreterParams, InterpreterResult, InterpreterType, Transaction};

/// BlockDataProvider provides functions to get block's hash from chain.
///
/// Block data(only hash) are required to cita-vm from externalize database.
pub trait BlockDataProvider: Send + Sync {
    /// Function get_block_hash returns the block_hash of the specific block.
    fn get_block_hash(&self, number: &U256) -> H256;
}

/// BlockDataProviderMock is a mock for BlockDataProvider. We could use it in
/// tests or demos.
#[derive(Default)]
pub struct BlockDataProviderMock {
    data: HashMap<U256, H256>,
}

impl BlockDataProviderMock {
    /// Set blockhash for a specific block.
    pub fn set(&mut self, number: U256, hash: H256) {
        self.data.insert(number, hash);
    }
}

/// Impl.
impl BlockDataProvider for BlockDataProviderMock {
    fn get_block_hash(&self, number: &U256) -> H256 {
        *self.data.get(number).unwrap_or(&H256::zero())
    }
}

/// An implemention for evm::DataProvider
pub struct DataProvider<B> {
    pub block_provider: Arc<dyn BlockDataProvider>,
    pub state_provider: Arc<RefCell<State<B>>>,
    pub store: Arc<RefCell<Store>>,
}

impl<B: DB> DataProvider<B> {
    /// Create a new instance. It's obvious.
    pub fn new(b: Arc<dyn BlockDataProvider>, s: Arc<RefCell<State<B>>>, store: Arc<RefCell<Store>>) -> Self {
        DataProvider {
            block_provider: b,
            state_provider: s,
            store,
        }
    }
}

impl<B: DB + 'static> evm::DataProvider for DataProvider<B> {
    fn get_balance(&self, address: &Address) -> U256 {
        self.state_provider
            .borrow_mut()
            .balance(address)
            .unwrap_or_else(|_| U256::zero())
    }

    fn add_refund(&mut self, address: &Address, n: u64) {
        log::debug!("ext.add_refund {:?} {}", address, n);
        self.store
            .borrow_mut()
            .refund
            .entry(*address)
            .and_modify(|v| *v += n)
            .or_insert(n);
    }

    fn sub_refund(&mut self, address: &Address, n: u64) {
        log::debug!("ext.sub_refund {:?} {}", address, n);
        self.store
            .borrow_mut()
            .refund
            .entry(*address)
            .and_modify(|v| *v -= n)
            .or_insert(n);
    }

    fn get_refund(&self, address: &Address) -> u64 {
        self.store.borrow_mut().refund.get(address).map_or(0, |v| *v)
    }

    fn get_code_size(&self, address: &Address) -> u64 {
        self.state_provider.borrow_mut().code_size(address).unwrap_or(0) as u64
    }

    fn get_code(&self, address: &Address) -> Vec<u8> {
        self.state_provider
            .borrow_mut()
            .code(address)
            .unwrap_or_else(|_| vec![])
    }

    fn get_code_hash(&self, address: &Address) -> H256 {
        self.state_provider
            .borrow_mut()
            .code_hash(address)
            .unwrap_or_else(|_| H256::zero())
    }

    fn get_block_hash(&self, number: &U256) -> H256 {
        self.block_provider.get_block_hash(number)
    }

    fn get_storage(&self, address: &Address, key: &H256) -> H256 {
        self.state_provider
            .borrow_mut()
            .get_storage(address, key)
            .unwrap_or_else(|_| H256::zero())
    }

    fn set_storage(&mut self, address: &Address, key: H256, value: H256) {
        let a = self.get_storage(address, &key);
        self.store
            .borrow_mut()
            .origin
            .entry(*address)
            .or_insert_with(HashMap::new)
            .entry(key)
            .or_insert(a);
        if let Err(e) = self.state_provider.borrow_mut().set_storage(address, key, value) {
            panic!("{}", e);
        }
    }

    fn get_storage_origin(&self, address: &Address, key: &H256) -> H256 {
        self.store.borrow_mut().used(address.clone());
        match self.store.borrow_mut().origin.get(address) {
            Some(account) => match account.get(key) {
                Some(val) => *val,
                None => self.get_storage(address, key),
            },
            None => self.get_storage(address, key),
        }
    }

    fn set_storage_origin(&mut self, _address: &Address, _key: H256, _value: H256) {
        unimplemented!()
    }

    fn selfdestruct(&mut self, address: &Address, refund_to: &Address) -> bool {
        if self.store.borrow_mut().selfdestruct.contains(address) {
            return false;
        }
        self.store.borrow_mut().used(refund_to.clone());
        self.store.borrow_mut().selfdestruct.insert(address.clone());
        let b = self.get_balance(address);

        if address != refund_to {
            self.state_provider
                .borrow_mut()
                .transfer_balance(address, refund_to, b)
                .unwrap();
        } else {
            // Must ensure that the balance of address which is suicide is zero.
            self.state_provider.borrow_mut().sub_balance(address, b).unwrap();
        }
        true
    }

    fn sha3(&self, data: &[u8]) -> H256 {
        From::from(&common::hash::summary(data)[..])
    }

    fn is_empty(&self, address: &Address) -> bool {
        self.state_provider.borrow_mut().is_empty(address).unwrap_or(false)
    }

    fn call(&self, opcode: evm::OpCode, params: InterpreterParams) -> (Result<InterpreterResult, evm::Error>) {
        match opcode {
            evm::OpCode::CALL | evm::OpCode::CALLCODE | evm::OpCode::DELEGATECALL | evm::OpCode::STATICCALL => {
                self.store.borrow_mut().used(params.address);
                let r = call(
                    self.block_provider.clone(),
                    self.state_provider.clone(),
                    self.store.clone(),
                    &params,
                );
                r.or(Err(evm::Error::CallError))
            }
            evm::OpCode::CREATE | evm::OpCode::CREATE2 => {
                let mut request = params.clone();
                request.nonce = self
                    .state_provider
                    .borrow_mut()
                    .nonce(&request.sender)
                    .or(Err(evm::Error::CallError))?;
                // Must inc nonce for sender
                // See: https://github.com/ethereum/EIPs/blob/master/EIPS/eip-161.md
                self.state_provider
                    .borrow_mut()
                    .inc_nonce(&request.sender)
                    .or(Err(evm::Error::CallError))?;
                let r = match opcode {
                    evm::OpCode::CREATE => create(
                        self.block_provider.clone(),
                        self.state_provider.clone(),
                        self.store.clone(),
                        &request,
                        CreateKind::FromAddressAndNonce,
                    ),
                    evm::OpCode::CREATE2 => create(
                        self.block_provider.clone(),
                        self.state_provider.clone(),
                        self.store.clone(),
                        &request,
                        CreateKind::FromSaltAndCodeHash,
                    ),
                    _ => unimplemented!(),
                }
                .or(Err(evm::Error::CallError));
                log::debug!("ext.create.result = {:?}", r);
                r
            }
            _ => unimplemented!(),
        }
    }
}

/// Store storages shared datas.
#[derive(Clone, Default)]
pub struct Store {
    pub refund: HashMap<Address, u64>,                 // For record refunds
    pub origin: HashMap<Address, HashMap<H256, H256>>, // For record origin value
    pub selfdestruct: HashSet<Address>,                // For self destruct
    // Field inused used for garbage collection.
    //
    // Test:
    //   ./tests/jsondata/GeneralStateTests/stSStoreTest/sstore_combinations_initial0.json
    //   ./tests/jsondata/GeneralStateTests/stSStoreTest/sstore_combinations_initial1.json
    //   ./tests/jsondata/GeneralStateTests/stSStoreTest/sstore_combinations_initial2.json
    pub inused: HashSet<Address>,
    pub context: Context,
    pub cfg: Config,
}

impl Store {
    /// Merge with sub store.
    pub fn merge(&mut self, other: Arc<RefCell<Self>>) {
        self.refund = other.borrow().refund.clone();
        self.origin = other.borrow().origin.clone();
        self.selfdestruct = other.borrow().selfdestruct.clone();
        self.inused = other.borrow().inused.clone();
    }

    /// When a account has been read or write, record a log
    /// to prove that it has dose.
    pub fn used(&mut self, address: Address) {
        self.inused.insert(address);
    }
}

/// Returns new address created from address and nonce.
pub fn create_address_from_address_and_nonce(address: &Address, nonce: &U256) -> Address {
    let mut stream = RlpStream::new_list(2);
    stream.append(address);
    stream.append(nonce);
    Address::from(H256::from(common::hash::summary(stream.as_raw()).as_slice()))
}

/// Returns new address created from sender salt and code hash.
/// See: EIP 1014.
pub fn create_address_from_salt_and_code_hash(address: &Address, salt: H256, code: Vec<u8>) -> Address {
    let code_hash = &common::hash::summary(&code[..])[..];
    let mut buffer = [0u8; 1 + 20 + 32 + 32];
    buffer[0] = 0xff;
    buffer[1..=20].copy_from_slice(&address[..]);
    buffer[(1 + 20)..(1 + 20 + 32)].copy_from_slice(&salt[..]);
    buffer[(1 + 20 + 32)..].copy_from_slice(code_hash);
    Address::from(H256::from(common::hash::summary(&buffer[..]).as_slice()))
}

/// A selector for func create_address_from_address_and_nonce() and
/// create_address_from_salt_and_code_hash()
pub enum CreateKind {
    FromAddressAndNonce, // use create_address_from_address_and_nonce
    FromSaltAndCodeHash, // use create_address_from_salt_and_code_hash
}

/// If a contract creation is attempted, due to either a creation transaction
/// or the CREATE (or future CREATE2) opcode, and the destination address
/// already has either nonzero nonce, or nonempty code, then the creation
/// throws immediately, with exactly the same behavior as would arise if the
/// first byte in the init code were an invalid opcode. This applies
/// retroactively starting from genesis.
///
/// See: EIP 684
pub fn can_create<B: DB + 'static>(
    state_provider: Arc<RefCell<state::State<B>>>,
    address: &Address,
) -> Result<bool, Error> {
    let a = state_provider.borrow_mut().nonce(&address)?;
    let b = state_provider.borrow_mut().code(&address)?;
    Ok(a.is_zero() && b.is_empty())
}

// There are two payment: fixed value for transcation and mutable value
// for input data. If is_create, another G_CREATE gas required.
//
// gas_prepare = 21000 + (68 or 4 per byte) + (32000 if tx.to == 0)
fn get_gas_prepare(request: &InterpreterParams) -> u64 {
    let mut gas_prepare: u64 = 0;
    gas_prepare += G_TRANSACTION;
    if request.is_create {
        gas_prepare += G_CREATE
    }
    for i in &request.input {
        if i == &0u8 {
            gas_prepare += G_TX_DATA_ZERO
        } else {
            gas_prepare += G_TX_DATA_NON_ZERO
        }
    }
    gas_prepare
}

/// Function get_refund returns the real ammount to refund for a transaction.
fn get_refund(store: Arc<RefCell<Store>>, request: &InterpreterParams, gas_left: u64) -> u64 {
    let refunds_bound = match store.borrow().refund.get(&request.origin) {
        Some(&data) => data,
        None => 0u64,
    };
    // Get real ammount to refund
    std::cmp::min(refunds_bound, (request.gas_limit - gas_left) >> 1)
}

/// Liquidtion for a transaction.
fn clear<B: DB + 'static>(
    state_provider: Arc<RefCell<state::State<B>>>,
    store: Arc<RefCell<Store>>,
    request: &InterpreterParams,
    gas_left: u64,
    refund: u64,
) -> Result<(), Error> {
    state_provider
        .borrow_mut()
        .add_balance(&request.sender, request.gas_price * (gas_left + refund))?;
    state_provider.borrow_mut().add_balance(
        &store.borrow().context.coinbase,
        request.gas_price * (request.gas_limit - gas_left - refund),
    )?;
    Ok(())
}

/// Mutable configs in cita-vm's execution.
#[derive(Clone, Debug)]
pub struct Config {
    pub block_gas_limit: u64, // gas limit for a block.
    pub check_nonce: bool,
    pub cfg_evm: evm::InterpreterConf,
    pub cfg_riscv: riscv::InterpreterConf,
}

impl Default for Config {
    fn default() -> Self {
        let mut cfg_evm = evm::InterpreterConf::default();
        cfg_evm.eip1283 = true;
        let cfg_riscv = riscv::InterpreterConf::default();
        Config {
            block_gas_limit: 8_000_000,
            check_nonce: true,
            cfg_evm,
            cfg_riscv,
        }
    }
}

/// Function call_pure enters into the specific contract with no check or checkpoints.
fn call_pure<B: DB + 'static>(
    block_provider: Arc<dyn BlockDataProvider>,
    state_provider: Arc<RefCell<state::State<B>>>,
    store: Arc<RefCell<Store>>,
    iparams: &InterpreterParams,
) -> Result<InterpreterResult, Error> {
    let context = store.borrow().context.clone();
    let cfg = store.borrow().cfg.clone();
    let iparams = iparams.clone();
    let data_provider = DataProvider::new(block_provider.clone(), state_provider.clone(), store.clone());
    // Transfer value
    if !iparams.disable_transfer_value {
        state_provider
            .borrow_mut()
            .transfer_balance(&iparams.sender, &iparams.receiver, iparams.value)?;
    }
    // Execute pre-compiled contracts.
    if native::contains(&iparams.contract.code_address) {
        let c = native::get(iparams.contract.code_address);
        let gas = c.required_gas(&iparams.input);
        if iparams.gas_limit < gas {
            return Err(Error::EVM(evm::Error::OutOfGas));
        }
        let r = c.run(&iparams.input);
        match r {
            Ok(ok) => {
                return Ok(InterpreterResult::Normal(ok, iparams.gas_limit - gas, vec![]));
            }
            Err(e) => return Err(e.into()),
        }
    }

    // Run
    match iparams.itype {
        InterpreterType::EVM => {
            let mut it = evm::Interpreter::new(context, cfg.cfg_evm, Box::new(data_provider), iparams);
            Ok(it.run()?)
        }
        InterpreterType::RISCV => {
            let mut it = riscv::Interpreter::new(context, cfg.cfg_riscv, iparams, Rc::new(RefCell::new(data_provider)));
            Ok(it.run()?)
        }
    }
}

/// Function call enters into the specific contract.
fn call<B: DB + 'static>(
    block_provider: Arc<dyn BlockDataProvider>,
    state_provider: Arc<RefCell<state::State<B>>>,
    store: Arc<RefCell<Store>>,
    iparams: &InterpreterParams,
) -> Result<InterpreterResult, Error> {
    log::debug!("call iparams={:?}", iparams);
    // Ensure balance
    if !iparams.disable_transfer_value && state_provider.borrow_mut().balance(&iparams.sender)? < iparams.value {
        return Err(Error::NotEnoughBalance);
    }
    // Run
    state_provider.borrow_mut().checkpoint();
    let store_son = Arc::new(RefCell::new(store.borrow_mut().clone()));
    let r = call_pure(
        block_provider.clone(),
        state_provider.clone(),
        store_son.clone(),
        iparams,
    );
    log::debug!("call result={:?}", r);
    match r {
        Ok(InterpreterResult::Normal(output, gas_left, logs)) => {
            state_provider.borrow_mut().discard_checkpoint();
            store.borrow_mut().merge(store_son);
            Ok(InterpreterResult::Normal(output, gas_left, logs))
        }
        Ok(InterpreterResult::Revert(output, gas_left)) => {
            state_provider.borrow_mut().revert_checkpoint();
            Ok(InterpreterResult::Revert(output, gas_left))
        }
        Err(e) => {
            state_provider.borrow_mut().revert_checkpoint();
            Err(e)
        }
        _ => unimplemented!(),
    }
}

/// Function create creates a new contract.
fn create<B: DB + 'static>(
    block_provider: Arc<dyn BlockDataProvider>,
    state_provider: Arc<RefCell<state::State<B>>>,
    store: Arc<RefCell<Store>>,
    iparams: &InterpreterParams,
    create_kind: CreateKind,
) -> Result<InterpreterResult, Error> {
    log::debug!("create iparams={:?}", iparams);
    let address = match create_kind {
        CreateKind::FromAddressAndNonce => {
            // Generate new address created from address, nonce
            create_address_from_address_and_nonce(&iparams.sender, &iparams.nonce)
        }
        CreateKind::FromSaltAndCodeHash => {
            // Generate new address created from sender salt and code hash
            create_address_from_salt_and_code_hash(&iparams.sender, iparams.extra, iparams.input.clone())
        }
    };
    log::debug!("create address={:?}", address);
    // Ensure there's no existing contract already at the designated address
    if !can_create(state_provider.clone(), &address)? {
        return Err(Error::ContractAlreadyExist);
    }

    // Just save the code at account's code field.
    if iparams.itype != InterpreterType::EVM {
        state_provider.borrow_mut().set_code(&address, iparams.input.clone())?;
        return Ok(InterpreterResult::Create(vec![], iparams.gas_limit, vec![], address));
    }

    // Make a checkpoint here
    state_provider.borrow_mut().checkpoint();
    // Create a new contract
    let balance = state_provider.borrow_mut().balance(&address)?;
    state_provider.borrow_mut().new_contract(
        &address,
        balance,
        // The init nonce for a new contract is one, see above documents.
        U256::one(),
        // The init code should be none. Consider a situation: ContractA will create
        // ContractB with address 0x1ff...fff, but ContractB's init code contains some
        // op like "get code hash from 0x1ff..fff or get code size form 0x1ff...fff",
        // The right result should be "summary(none)" and "0".
        vec![],
    );
    let mut jparams = iparams.clone();
    jparams.address = address;
    jparams.receiver = address;
    jparams.is_create = false;
    jparams.input = vec![];
    jparams.contract = Contract {
        code_address: address,
        code_data: iparams.input.clone(),
    };
    let r = call(block_provider.clone(), state_provider.clone(), store.clone(), &jparams);
    match r {
        Ok(InterpreterResult::Normal(output, gas_left, logs)) => {
            // Ensure code size
            if output.len() as u64 > MAX_CREATE_CODE_SIZE {
                state_provider.borrow_mut().revert_checkpoint();
                return Err(Error::ExccedMaxCodeSize);
            }
            // Pay every byte returnd from CREATE
            let gas_code_deposit: u64 = G_CODE_DEPOSIT * output.len() as u64;
            if gas_left < gas_code_deposit {
                state_provider.borrow_mut().revert_checkpoint();
                return Err(Error::EVM(evm::Error::OutOfGas));
            }
            let gas_left = gas_left - gas_code_deposit;
            state_provider.borrow_mut().set_code(&address, output.clone())?;
            state_provider.borrow_mut().discard_checkpoint();
            let r = Ok(InterpreterResult::Create(output, gas_left, logs, address));
            log::debug!("create result={:?}", r);
            log::debug!("create gas_left={:?}", gas_left);
            r
        }
        Ok(InterpreterResult::Revert(output, gas_left)) => {
            state_provider.borrow_mut().revert_checkpoint();
            let r = Ok(InterpreterResult::Revert(output, gas_left));
            log::debug!("create gas_left={:?}", gas_left);
            log::debug!("create result={:?}", r);
            r
        }
        Err(e) => {
            log::debug!("create err={:?}", e);
            state_provider.borrow_mut().revert_checkpoint();
            Err(e)
        }
        _ => unimplemented!(),
    }
}

const G_TX_DATA_ZERO: u64 = 4; // Paid for every zero byte of data or code for a transaction
const G_TX_DATA_NON_ZERO: u64 = 68; // Paid for every non-zero byte of data or code for a transaction
const G_TRANSACTION: u64 = 21000; // Paid for every transaction
const G_CREATE: u64 = 32000; // Paid for contract create
const G_CODE_DEPOSIT: u64 = 200; // Paid per byte for a CREATE operation to succeed in placing code into state.
const MAX_CREATE_CODE_SIZE: u64 = 24576; // See: https://github.com/ethereum/EIPs/issues/659

/// Reinterpret tx to interpreter params.
fn reinterpret_tx<B: DB + 'static>(
    tx: Transaction,
    state_provider: Arc<RefCell<state::State<B>>>,
) -> InterpreterParams {
    let mut iparams = InterpreterParams::default();
    iparams.origin = tx.from;
    iparams.sender = tx.from;
    match tx.to {
        Some(data) => {
            iparams.receiver = data;
            iparams.address = data;
            iparams.contract = Contract {
                code_address: data,
                code_data: state_provider.borrow_mut().code(&data).unwrap_or_default(),
            };
        }
        None => {
            iparams.is_create = true;
        }
    }
    iparams.gas_price = tx.gas_price;
    iparams.gas_limit = tx.gas_limit;
    iparams.value = tx.value;
    iparams.input = tx.input;
    iparams.nonce = tx.nonce;
    iparams.itype = tx.itype;
    iparams
}

pub struct Executive<B> {
    pub block_provider: Arc<dyn BlockDataProvider>,
    pub state_provider: Arc<RefCell<state::State<B>>>,
    pub config: Config,
}

impl<B: DB + 'static> Executive<B> {
    pub fn new(block_provider: Arc<dyn BlockDataProvider>, state_provider: state::State<B>, config: Config) -> Self {
        Self {
            block_provider,
            state_provider: Arc::new(RefCell::new(state_provider)),
            config,
        }
    }

    pub fn exec(&self, context: Context, tx: Transaction) -> Result<InterpreterResult, Error> {
        let iparams = &reinterpret_tx(tx, self.state_provider.clone());
        // Ensure gas < block_gas_limit
        if self.config.block_gas_limit > G_TRANSACTION && iparams.gas_limit > self.config.block_gas_limit {
            return Err(Error::ExccedMaxBlockGasLimit);
        }
        if self.config.check_nonce {
            // Ensure nonce
            if iparams.nonce != self.state_provider.borrow_mut().nonce(&iparams.sender)? {
                return Err(Error::InvalidNonce);
            }
        }
        // Ensure gas
        let gas_prepare = get_gas_prepare(iparams);
        if iparams.gas_limit < gas_prepare {
            return Err(Error::NotEnoughBaseGas);
        }
        // Ensure value
        let gas_prepay = iparams.gas_price * iparams.gas_limit;
        if self.state_provider.borrow_mut().balance(&iparams.sender)? < gas_prepay + iparams.value {
            return Err(Error::NotEnoughBalance);
        }
        // Pay intrinsic gas
        self.state_provider
            .borrow_mut()
            .sub_balance(&iparams.sender, gas_prepay)?;
        // Increament the nonce for the next transaction
        self.state_provider.borrow_mut().inc_nonce(&iparams.sender)?;
        // Init the store for the transaction
        let mut store = Store::default();
        store.cfg = self.config.clone();
        store.context = context.clone();
        store.used(iparams.receiver);
        let store = Arc::new(RefCell::new(store));
        // Create a sub request
        let mut jparams = iparams.clone();
        jparams.gas_limit = iparams.gas_limit - gas_prepare;
        let r = if iparams.is_create {
            create(
                self.block_provider.clone(),
                self.state_provider.clone(),
                store.clone(),
                &jparams,
                CreateKind::FromAddressAndNonce,
            )
        } else {
            call(
                self.block_provider.clone(),
                self.state_provider.clone(),
                store.clone(),
                &jparams,
            )
        };
        // Finalize
        log::debug!("exec result={:?}", r);
        match r {
            Ok(InterpreterResult::Normal(output, gas_left, logs)) => {
                log::debug!("exec gas_left={:?}", gas_left);
                let refund = get_refund(store.clone(), &iparams, gas_left);
                log::debug!("exec refund={:?}", refund);
                clear(self.state_provider.clone(), store.clone(), &iparams, gas_left, refund)?;
                // Handle self destruct: Kill it.
                // Note: must after ends of the transaction.
                for e in store.borrow_mut().selfdestruct.drain() {
                    self.state_provider.borrow_mut().kill_contract(&e)
                }
                self.state_provider
                    .borrow_mut()
                    .kill_garbage(&store.borrow().inused.clone());
                Ok(InterpreterResult::Normal(output, gas_left, logs))
            }
            Ok(InterpreterResult::Revert(output, gas_left)) => {
                log::debug!("exec gas_left={:?}", gas_left);
                clear(self.state_provider.clone(), store.clone(), &iparams, gas_left, 0)?;
                self.state_provider
                    .borrow_mut()
                    .kill_garbage(&store.borrow().inused.clone());
                Ok(InterpreterResult::Revert(output, gas_left))
            }
            Ok(InterpreterResult::Create(output, gas_left, logs, addr)) => {
                log::debug!("exec gas_left={:?}", gas_left);
                let refund = get_refund(store.clone(), &iparams, gas_left);
                log::debug!("exec refund={:?}", refund);
                clear(self.state_provider.clone(), store.clone(), &iparams, gas_left, refund)?;
                for e in store.borrow_mut().selfdestruct.drain() {
                    self.state_provider.borrow_mut().kill_contract(&e)
                }
                self.state_provider
                    .borrow_mut()
                    .kill_garbage(&store.borrow().inused.clone());
                Ok(InterpreterResult::Create(output, gas_left, logs, addr))
            }
            Err(e) => {
                // When error, coinbase eats all gas as it's price, yummy.
                clear(self.state_provider.clone(), store.clone(), &iparams, 0, 0)?;
                self.state_provider
                    .borrow_mut()
                    .kill_garbage(&store.borrow().inused.clone());
                Err(e)
            }
        }
    }

    /// Handle the call request in read only mode.
    /// Note:
    ///   1) tx.to shouldn't be none
    ///   2) tx.nonce is just omited
    ///   3) tx.value must be 0. This is due to solidity's check.
    ///
    /// This function is similar with `exec`, but all check & checkpoints are removed.
    pub fn exec_static(&self, context: Context, tx: Transaction) -> Result<InterpreterResult, Error> {
        if tx.to.is_none() {
            return Err(Error::CreateInStaticCall);
        }
        let mut iparams = reinterpret_tx(tx, self.state_provider.clone());
        iparams.read_only = true;
        iparams.disable_transfer_value = true;
        let mut store = Store::default();
        store.cfg = self.config.clone();
        store.context = context.clone();
        let store = Arc::new(RefCell::new(store));
        call_pure(
            self.block_provider.clone(),
            self.state_provider.clone(),
            store.clone(),
            &iparams,
        )
    }

    pub fn commit(&self) -> Result<H256, Error> {
        self.state_provider.borrow_mut().commit()?;
        Ok(self.state_provider.borrow_mut().root)
    }
}
