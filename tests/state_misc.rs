use std::cell::RefCell;
use std::sync::Arc;

use cita_vm::state::StateObjectInfo;
use numext_fixed_hash::H160 as Address;
use numext_fixed_uint::U256;

#[test]
fn test_state_misc00() {
    let db = Arc::new(cita_vm::state::MemoryDB::new(false));
    let mut state = cita_vm::state::State::new(db.clone()).unwrap();

    state.new_contract(
        &Address::from_hex_str("0x2000000000000000000000000000000000000000").unwrap(),
        U256::from(100_000 as u32),
        U256::from(1 as u32),
        hex::decode("").unwrap(),
    );
    state.new_contract(
        &Address::from_hex_str("0x1000000000000000000000000000000000000000").unwrap(),
        U256::from(200_000 as u32),
        U256::from(1 as u32),
        vec![],
    );
    state.commit().unwrap();
    let root0 = state.root.clone();

    let block_data_provider: Arc<cita_vm::BlockDataProvider> = Arc::new(cita_vm::BlockDataProviderMock::default());
    let state_data_provider = Arc::new(RefCell::new(state));
    let context = cita_vm::evm::Context::default();
    let config = cita_vm::Config::default();

    let tx = cita_vm::Transaction {
        from: Address::from_hex_str("0x1000000000000000000000000000000000000000").unwrap(),
        to: Some(Address::from_hex_str("0x2000000000000000000000000000000000000000").unwrap()),
        value: U256::from(5 as u32),
        nonce: U256::from(1 as u32),
        gas_limit: 80000,
        gas_price: U256::from(1 as u32),
        input: hex::decode("").unwrap(),
    };
    let _ = cita_vm::exec(
        block_data_provider.clone(),
        state_data_provider.clone(),
        context.clone(),
        config.clone(),
        tx,
    );
    state_data_provider.borrow_mut().commit().unwrap();

    assert_eq!(
        state_data_provider
            .borrow_mut()
            .balance(&Address::from_hex_str("0x2000000000000000000000000000000000000000").unwrap())
            .unwrap(),
        U256::from(100_005 as u32)
    );
    let mut ur_state = cita_vm::state::State::from_existing(db, root0).unwrap();
    let b = ur_state
        .balance(&Address::from_hex_str("0x2000000000000000000000000000000000000000").unwrap())
        .unwrap();
    assert_eq!(b, U256::from(100_000 as u32));
}
