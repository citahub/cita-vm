use std::cell::RefCell;
use std::sync::Arc;

use cita_trie::DB;
use ethereum_types::{Address, H256, U256};
use evm::InterpreterParams;
use hashbrown::{HashMap, HashSet};
use log::debug;
use rlp::RlpStream;

use crate::common;
use crate::err;
use crate::evm;
use crate::native;
use crate::state::{self, State, StateObjectInfo};

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

/// Store storages shared datas.
#[derive(Clone, Default, Debug)]
pub struct Store {
    refund: HashMap<Address, u64>,                 // For record refunds
    origin: HashMap<Address, HashMap<H256, H256>>, // For record origin value
    selfdestruct: HashSet<Address>,                // For self destruct
    // Field inused used for garbage collection.
    //
    // Test:
    //   ./tests/jsondata/GeneralStateTests/stSStoreTest/sstore_combinations_initial0.json
    //   ./tests/jsondata/GeneralStateTests/stSStoreTest/sstore_combinations_initial1.json
    //   ./tests/jsondata/GeneralStateTests/stSStoreTest/sstore_combinations_initial2.json
    inused: HashSet<Address>,
    evm_context: evm::Context,
    evm_cfg: evm::InterpreterConf,
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
        if address == Address::zero() {
            return;
        }
        self.inused.insert(address);
    }
}

/// An implemention for evm::DataProvider
pub struct DataProvider<B> {
    block_provider: Arc<BlockDataProvider>,
    state_provider: Arc<RefCell<State<B>>>,
    store: Arc<RefCell<Store>>,
}

impl<B: DB> DataProvider<B> {
    /// Create a new instance. It's obvious.
    pub fn new(b: Arc<BlockDataProvider>, s: Arc<RefCell<State<B>>>, store: Arc<RefCell<Store>>) -> Self {
        DataProvider {
            block_provider: b,
            state_provider: s,
            store,
        }
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

/// Returns the default interpreter configs for Constantinople.
pub fn get_interpreter_conf() -> evm::InterpreterConf {
    let mut evm_cfg = evm::InterpreterConf::default();
    evm_cfg.eip1283 = false;
    evm_cfg
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
) -> Result<bool, err::Error> {
    let a = state_provider.borrow_mut().nonce(&address)?;
    let b = state_provider.borrow_mut().code(&address)?;
    Ok(a.is_zero() && b.is_empty())
}

// There are two payment: fixed value for transcation and mutable value
// for input data. If is_create, another G_CREATE gas required.
//
// gas_prepare = 21000 + (68 or 4 per byte) + (32000 if tx.to == 0)
pub fn get_gas_prepare(request: &InterpreterParams) -> u64 {
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
pub fn get_refund(store: Arc<RefCell<Store>>, request: &InterpreterParams, gas_left: u64) -> u64 {
    let refunds_bound = match store.borrow().refund.get(&request.origin) {
        Some(&data) => data,
        None => 0u64,
    };
    // Get real ammount to refund
    std::cmp::min(refunds_bound, (request.gas_limit - gas_left) >> 1)
}

/// Liquidtion for a transaction.
pub fn clear<B: DB + 'static>(
    state_provider: Arc<RefCell<state::State<B>>>,
    store: Arc<RefCell<Store>>,
    request: &InterpreterParams,
    gas_left: u64,
    refund: u64,
) -> Result<(), err::Error> {
    state_provider
        .borrow_mut()
        .add_balance(&request.sender, request.gas_price * (gas_left + refund))?;
    state_provider.borrow_mut().add_balance(
        &store.borrow().evm_context.coinbase,
        request.gas_price * (request.gas_limit - gas_left - refund),
    )?;
    Ok(())
}

/// Mutable configs in cita-vm's execution.
#[derive(Clone, Debug)]
pub struct Config {
    pub block_gas_limit: u64, // gas limit for a block.
    pub check_nonce: bool,
    pub check_balance: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            block_gas_limit: 8_000_000,
            check_nonce: false,
            check_balance: true,
        }
    }
}

/// Function call_pure enters into the specific contract with no check or checkpoints.
fn call_pure<B: DB + 'static>(
    block_provider: Arc<BlockDataProvider>,
    state_provider: Arc<RefCell<state::State<B>>>,
    store: Arc<RefCell<Store>>,
    request: &InterpreterParams,
) -> Result<evm::InterpreterResult, err::Error> {
    let evm_context = store.borrow().evm_context.clone();
    let evm_cfg = store.borrow().evm_cfg.clone();
    let evm_params = request.clone();
    let evm_data_provider = DataProvider::new(block_provider.clone(), state_provider.clone(), store.clone());
    // Transfer value
    if !request.disable_transfer_value {
        state_provider
            .borrow_mut()
            .transfer_balance(&request.sender, &request.receiver, request.value)?;
    }

    // Execute pre-compiled contracts.
    if native::contains(&request.contract.code_address) {
        let c = native::get(request.contract.code_address);
        let gas = c.required_gas(&request.input);
        if request.gas_limit < gas {
            return Err(err::Error::Evm(evm::Error::OutOfGas));
        }
        let r = c.run(&request.input);
        match r {
            Ok(ok) => {
                return Ok(evm::InterpreterResult::Normal(ok, request.gas_limit - gas, vec![]));
            }
            Err(e) => return Err(e),
        }
    }
    // Run
    let mut evm_it = evm::Interpreter::new(evm_context, evm_cfg, Box::new(evm_data_provider), evm_params);
    Ok(evm_it.run()?)
}

/// Function call enters into the specific contract.
fn call<B: DB + 'static>(
    block_provider: Arc<BlockDataProvider>,
    state_provider: Arc<RefCell<state::State<B>>>,
    store: Arc<RefCell<Store>>,
    request: &InterpreterParams,
) -> Result<evm::InterpreterResult, err::Error> {
    // Here not need check twice,becauce prepay is subed ,but need think call_static
    /*if !request.disable_transfer_value && state_provider.borrow_mut().balance(&request.sender)? < request.value {
        return Err(err::Error::NotEnoughBalance);
    }*/
    // Run
    state_provider.borrow_mut().checkpoint();
    let store_son = Arc::new(RefCell::new(store.borrow_mut().clone()));
    let r = call_pure(
        block_provider.clone(),
        state_provider.clone(),
        store_son.clone(),
        request,
    );
    debug!("call result={:?}", r);
    match r {
        Ok(evm::InterpreterResult::Normal(output, gas_left, logs)) => {
            state_provider.borrow_mut().discard_checkpoint();
            store.borrow_mut().merge(store_son);
            Ok(evm::InterpreterResult::Normal(output, gas_left, logs))
        }
        Ok(evm::InterpreterResult::Revert(output, gas_left)) => {
            state_provider.borrow_mut().revert_checkpoint();
            Ok(evm::InterpreterResult::Revert(output, gas_left))
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
    block_provider: Arc<BlockDataProvider>,
    state_provider: Arc<RefCell<state::State<B>>>,
    store: Arc<RefCell<Store>>,
    request: &InterpreterParams,
    create_kind: CreateKind,
) -> Result<evm::InterpreterResult, err::Error> {
    debug!("create request={:?}", request);
    let address = match create_kind {
        CreateKind::FromAddressAndNonce => {
            // Generate new address created from address, nonce
            create_address_from_address_and_nonce(&request.sender, &request.nonce)
        }
        CreateKind::FromSaltAndCodeHash => {
            // Generate new address created from sender salt and code hash
            create_address_from_salt_and_code_hash(&request.sender, request.extra, request.input.clone())
        }
    };
    debug!("create address={:?}", address);
    // Ensure there's no existing contract already at the designated address
    if !can_create(state_provider.clone(), &address)? {
        return Err(err::Error::ContractAlreadyExist);
    }
    // Make a checkpoint here
    state_provider.borrow_mut().checkpoint();
    // Create a new contract
    let balance = state_provider.borrow_mut().balance(&address)?;
    state_provider.borrow_mut().new_contract(
        &address,
        balance,
        // The init nonce for a new contract is one, see above documents.
        U256::zero(),
        // The init code should be none. Consider a situation: ContractA will create
        // ContractB with address 0x1ff...fff, but ContractB's init code contains some
        // op like "get code hash from 0x1ff..fff or get code size form 0x1ff...fff",
        // The right result should be "summary(none)" and "0".
        vec![],
    );
    let mut reqchan = request.clone();
    reqchan.address = address;
    reqchan.receiver = address;
    reqchan.is_create = false;
    reqchan.input = vec![];
    reqchan.contract = evm::Contract {
        code_address: address,
        code_data: request.input.clone(),
    };
    let r = call(block_provider.clone(), state_provider.clone(), store.clone(), &reqchan);
    match r {
        Ok(evm::InterpreterResult::Normal(output, gas_left, logs)) => {
            // Ensure code size
            if output.len() as u64 > MAX_CREATE_CODE_SIZE {
                state_provider.borrow_mut().revert_checkpoint();
                return Err(err::Error::ExccedMaxCodeSize);
            }
            // Pay every byte returnd from CREATE
            let gas_code_deposit: u64 = G_CODE_DEPOSIT * output.len() as u64;
            if gas_left < gas_code_deposit {
                state_provider.borrow_mut().revert_checkpoint();
                return Err(err::Error::Evm(evm::Error::OutOfGas));
            }
            let gas_left = gas_left - gas_code_deposit;
            state_provider.borrow_mut().set_code(&address, output.clone())?;
            state_provider.borrow_mut().discard_checkpoint();
            let r = Ok(evm::InterpreterResult::Create(output, gas_left, logs, address));
            debug!("create result={:?}", r);
            debug!("create gas_left={:?}", gas_left);
            r
        }
        Ok(evm::InterpreterResult::Revert(output, gas_left)) => {
            state_provider.borrow_mut().revert_checkpoint();
            let r = Ok(evm::InterpreterResult::Revert(output, gas_left));
            debug!("create gas_left={:?}", gas_left);
            debug!("create result={:?}", r);
            r
        }
        Err(e) => {
            debug!("create err={:?}", e);
            state_provider.borrow_mut().revert_checkpoint();
            Err(e)
        }
        _ => unimplemented!(),
    }
}

const G_TX_DATA_ZERO: u64 = 0; //4; // Paid for every zero byte of data or code for a transaction
const G_TX_DATA_NON_ZERO: u64 = 0; //68; // Paid for every non-zero byte of data or code for a transaction
const G_TRANSACTION: u64 = 21000; // Paid for every transaction
const G_CREATE: u64 = 32000; // Paid for contract create
const G_CODE_DEPOSIT: u64 = 200; // Paid per byte for a CREATE operation to succeed in placing code into state.
const MAX_CREATE_CODE_SIZE: u64 = std::u64::MAX; // See: https://github.com/ethereum/EIPs/issues/659

/// Transaction struct.
#[derive(Clone, Debug)]
pub struct Transaction {
    pub from: Address,
    pub to: Option<Address>, // Some for call and None for create.
    pub value: U256,
    pub nonce: U256,
    pub gas_limit: u64,
    pub gas_price: U256,
    pub input: Vec<u8>,
}

/// Reinterpret tx to interpreter params.
fn reinterpret_tx<B: DB + 'static>(
    tx: Transaction,
    state_provider: Arc<RefCell<state::State<B>>>,
) -> InterpreterParams {
    let mut request = evm::InterpreterParams::default();
    request.origin = tx.from;
    request.sender = tx.from;
    match tx.to {
        Some(data) => {
            request.receiver = data;
            request.address = data;
            request.contract = evm::Contract {
                code_address: data,
                code_data: state_provider.borrow_mut().code(&data).unwrap_or_default(),
            };
        }
        None => {
            request.is_create = true;
        }
    }
    request.gas_price = tx.gas_price;
    request.gas_limit = tx.gas_limit;
    request.value = tx.value;
    request.input = tx.input;
    request.nonce = tx.nonce;
    request
}

/// Execute the transaction from transaction pool
pub fn exec<B: DB + 'static>(
    block_provider: Arc<BlockDataProvider>,
    state_provider: Arc<RefCell<state::State<B>>>,
    evm_context: evm::Context,
    config: Config,
    tx: Transaction,
) -> Result<evm::InterpreterResult, err::Error> {
    let request = &mut reinterpret_tx(tx, state_provider.clone());
    // Ensure gas < block_gas_limit

    /* TODO : this judgement need be reconsider
     fi config.block_gas_limit > G_TRANSACTION && request.gas_limit > config.block_gas_limit {
        return Err(err::Error::ExccedMaxBlockGasLimit);
    }
    */

    if config.check_nonce {
        // Ensure nonce. for state test compatible
        //if request.nonce + 1 != state_provider.borrow_mut().nonce(&request.sender)? {
        if request.nonce != state_provider.borrow_mut().nonce(&request.sender)? {
            return Err(err::Error::InvalidNonce);
        }
    }
    /*else {
        request.nonce = state_provider.borrow_mut().nonce(&request.sender)?;
    }*/
    // Ensure gas
    let gas_prepare = get_gas_prepare(request);
    if request.gas_limit < gas_prepare {
        return Err(err::Error::NotEnoughBaseGas);
    }

    // Ensure value
    if config.check_balance {
        let gas_prepay = request.gas_price * request.gas_limit;
        if state_provider.borrow_mut().balance(&request.sender)? < gas_prepay + request.value {
            return Err(err::Error::NotEnoughBalance);
        }
        // Pay intrinsic gas
        state_provider.borrow_mut().sub_balance(&request.sender, gas_prepay)?;
    }

    // Increament the nonce for the next transaction
    // Comment for inc_nonce out of this exec
    state_provider.borrow_mut().inc_nonce(&request.sender)?;

    // Init the store for the transaction
    let mut store = Store::default();
    store.evm_cfg = get_interpreter_conf();
    store.evm_context = evm_context.clone();
    //store.used(request.receiver);
    let store = Arc::new(RefCell::new(store));
    // Create a sub request
    let mut reqchan = request.clone();
    reqchan.gas_limit = request.gas_limit - gas_prepare;
    if !config.check_balance {
        reqchan.disable_transfer_value = true;
    }
    let r = if request.is_create {
        create(
            block_provider.clone(),
            state_provider.clone(),
            store.clone(),
            &reqchan,
            CreateKind::FromAddressAndNonce,
        )
    } else {
        call(block_provider.clone(), state_provider.clone(), store.clone(), &reqchan)
    };
    // Finalize
    match r {
        Ok(evm::InterpreterResult::Normal(output, gas_left, logs)) => {
            if config.check_balance {
                let refund = get_refund(store.clone(), &request, gas_left);
                clear(state_provider.clone(), store.clone(), &request, gas_left, refund)?;
            }
            // Handle self destruct: Kill it.
            // Note: must after ends of the transaction.
            for e in store.borrow_mut().selfdestruct.drain() {
                state_provider.borrow_mut().kill_contract(&e)
            }
            state_provider.borrow_mut().kill_garbage(&store.borrow().inused.clone());
            Ok(evm::InterpreterResult::Normal(output, gas_left, logs))
        }
        Ok(evm::InterpreterResult::Revert(output, gas_left)) => {
            if config.check_balance {
                clear(state_provider.clone(), store.clone(), &request, gas_left, 0)?;
            }
            state_provider.borrow_mut().kill_garbage(&store.borrow().inused.clone());
            Ok(evm::InterpreterResult::Revert(output, gas_left))
        }
        Ok(evm::InterpreterResult::Create(output, gas_left, logs, addr)) => {
            if config.check_balance {
                let refund = get_refund(store.clone(), &request, gas_left);
                clear(state_provider.clone(), store.clone(), &request, gas_left, refund)?;
            }
            for e in store.borrow_mut().selfdestruct.drain() {
                state_provider.borrow_mut().kill_contract(&e)
            }
            state_provider.borrow_mut().kill_garbage(&store.borrow().inused.clone());
            Ok(evm::InterpreterResult::Create(output, gas_left, logs, addr))
        }
        Err(e) => {
            // When error, coinbase eats all gas as it's price, yummy.
            if config.check_balance {
                clear(state_provider.clone(), store.clone(), &request, 0, 0)?;
            }
            state_provider.borrow_mut().kill_garbage(&store.borrow().inused.clone());
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
#[allow(unused_variables)]
pub fn exec_static<B: DB + 'static>(
    block_provider: Arc<BlockDataProvider>,
    state_provider: Arc<RefCell<state::State<B>>>,
    evm_context: evm::Context,
    config: Config,
    tx: Transaction,
) -> Result<evm::InterpreterResult, err::Error> {
    if tx.to.is_none() {
        return Err(err::Error::CreateInStaticCall);
    }
    let mut request = reinterpret_tx(tx, state_provider.clone());
    request.read_only = true;
    request.disable_transfer_value = true;
    let mut store = Store::default();
    store.evm_cfg = get_interpreter_conf();
    store.evm_context = evm_context.clone();
    let store = Arc::new(RefCell::new(store));
    call_pure(block_provider.clone(), state_provider.clone(), store.clone(), &request)
}

pub struct Executive<B> {
    pub block_provider: Arc<BlockDataProvider>,
    pub state_provider: Arc<RefCell<state::State<B>>>,
    pub config: Config,
}

impl<B: DB + 'static> Executive<B> {
    pub fn new(block_provider: Arc<BlockDataProvider>, state_provider: state::State<B>, config: Config) -> Self {
        Self {
            block_provider,
            state_provider: Arc::new(RefCell::new(state_provider)),
            config,
        }
    }

    pub fn exec(&self, evm_context: evm::Context, tx: Transaction) -> Result<evm::InterpreterResult, err::Error> {
        /*exec(
            self.block_provider.clone(),
            self.state_provider.clone(),
            evm_context,
            self.config.clone(),
            tx.clone(),
        )*/

        // Bellow is saved for jsondata test
        let coinbase = evm_context.coinbase;
        let exec_result = exec(
            self.block_provider.clone(),
            self.state_provider.clone(),
            evm_context,
            self.config.clone(),
            tx.clone(),
        );
        match exec_result {
            Err(err::Error::ExccedMaxBlockGasLimit)
            | Err(err::Error::NotEnoughBaseGas)
            | Err(err::Error::InvalidNonce)
            | Err(err::Error::NotEnoughBalance) => {
                let balance = tx.gas_price * G_TRANSACTION;
                let account_balance = self.state_provider.borrow_mut().balance(&tx.from)?;
                let real = {
                    if balance > account_balance {
                        account_balance
                    } else {
                        balance
                    }
                };
                self.state_provider.borrow_mut().sub_balance(&tx.from, real)?;
                self.state_provider.borrow_mut().add_balance(&coinbase, real)?;
                self.state_provider.borrow_mut().inc_nonce(&tx.from)?;
            }
            Err(err::Error::Evm(_)) => {}
            Err(_) => {}
            Ok(_) => {}
        }
        exec_result
    }

    pub fn exec_static(
        block_provider: Arc<BlockDataProvider>,
        state_provider: state::State<B>,
        evm_context: evm::Context,
        config: Config,
        tx: Transaction,
    ) -> Result<evm::InterpreterResult, err::Error> {
        exec_static(
            block_provider,
            Arc::new(RefCell::new(state_provider)),
            evm_context,
            config,
            tx,
        )
    }

    pub fn commit(&self) -> Result<H256, err::Error> {
        self.state_provider.borrow_mut().commit()?;
        Ok(self.state_provider.borrow_mut().root)
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
        self.store
            .borrow_mut()
            .refund
            .entry(*address)
            .and_modify(|v| *v += n)
            .or_insert(n);
    }

    fn sub_refund(&mut self, address: &Address, n: u64) {
        debug!("ext.sub_refund {:?} {}", address, n);
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
        //self.store.borrow_mut().used(address.clone());
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
        //self.store.borrow_mut().used(refund_to.clone());
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

    fn exist(&self, address: &Address) -> bool {
        self.state_provider.borrow_mut().exist(address).unwrap_or(false)
    }

    fn call(
        &self,
        opcode: evm::OpCode,
        params: evm::InterpreterParams,
    ) -> (Result<evm::InterpreterResult, evm::Error>) {
        match opcode {
            evm::OpCode::CALL | evm::OpCode::CALLCODE | evm::OpCode::DELEGATECALL | evm::OpCode::STATICCALL => {
                //self.store.borrow_mut().used(params.address);
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
                debug!("ext.create.result = {:?}", r);
                r
            }
            _ => unimplemented!(),
        }
    }
}
