use ethereum_types::{Address, U256};
use log::debug;
use std::cell::RefCell;
use std::sync::Arc;

#[test]
fn test_solidity_simplestorage() {
    let _ = env_logger::builder().is_test(true).try_init();
    let db = cita_vm::state::MemoryDB::new();
    let mut state = cita_vm::state::State::new(db).unwrap();
    let code = "6080604052600436106049576000357c0100000000000000000000000000000\
                000000000000000000000000000900463ffffffff16806360fe47b114604e57\
                80636d4ce63c146078575b600080fd5b348015605957600080fd5b506076600\
                4803603810190808035906020019092919050505060a0565b005b3480156083\
                57600080fd5b50608a60aa565b6040518082815260200191505060405180910\
                390f35b8060008190555050565b600080549050905600a165627a7a72305820\
                99c66a25d59f0aa78f7ebc40748fa1d1fbc335d8d780f284841b30e0365acd9\
                60029";
    state.new_contract(
        &Address::from("0xBd770416a3345F91E4B34576cb804a576fa48EB1"),
        U256::from(10),
        U256::from(1),
        hex::decode(code).unwrap(),
    );
    state.new_contract(
        &Address::from("0x1000000000000000000000000000000000000000"),
        U256::from(1_000_000_000_000_000u64),
        U256::from(1),
        vec![],
    );

    let block_data_provider: Arc<Box<cita_vm::BlockDataProvider>> =
        Arc::new(Box::new(cita_vm::BlockDataProviderMock::default()));
    let state_data_provider = Arc::new(RefCell::new(state));
    let context = cita_vm::evm::Context::default();
    let config = cita_vm::Config::default();

    // SimpleStorage.set(42)
    let tx = cita_vm::Transaction {
        from: Address::from("0x1000000000000000000000000000000000000000"),
        to: Some(Address::from("0xBd770416a3345F91E4B34576cb804a576fa48EB1")),
        value: U256::from(0),
        nonce: U256::from(1),
        gas_limit: 80000,
        gas_price: U256::from(1),
        input: hex::decode("60fe47b1000000000000000000000000000000000000000000000000000000000000002a").unwrap(),
    };
    let _ = cita_vm::exec(
        block_data_provider.clone(),
        state_data_provider.clone(),
        context.clone(),
        config.clone(),
        tx,
    )
    .unwrap();

    // Send transaction: SimpleStorage.get() => 42
    let tx = cita_vm::Transaction {
        from: Address::from("0x1000000000000000000000000000000000000000"),
        to: Some(Address::from("0xBd770416a3345F91E4B34576cb804a576fa48EB1")),
        value: U256::from(0),
        nonce: U256::from(2),
        gas_limit: 80000,
        gas_price: U256::from(1),
        input: hex::decode("6d4ce63c").unwrap(),
    };
    let r = cita_vm::exec(
        block_data_provider.clone(),
        state_data_provider.clone(),
        context.clone(),
        config.clone(),
        tx,
    )
    .unwrap();
    match r {
        cita_vm::evm::InterpreterResult::Normal(output, _, _) => {
            assert_eq!(
                output,
                vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42]
            );
        }
        _ => panic!(""),
    }

    // Eth call: SimpleStorage.get() => 42
    let tx = cita_vm::Transaction {
        from: Address::from("0x1000000000000000000000000000000000000001"), // Omited in most cases.
        to: Some(Address::from("0xBd770416a3345F91E4B34576cb804a576fa48EB1")), // tx.to shouldn't be none.
        value: U256::from(0),       // tx.value must be 0. This is due to solidity's check.
        nonce: U256::from(123_456), // tx.nonce is just omited.
        gas_limit: 80000,           // Give me a large enougth value plz.
        gas_price: U256::from(1),   // Omited due to solidity's check.
        input: hex::decode("6d4ce63c").unwrap(),
    };
    let r = cita_vm::exec_static(
        block_data_provider.clone(),
        state_data_provider.clone(),
        context.clone(),
        config.clone(),
        tx,
    )
    .unwrap();
    debug!("{:?}", r);
    match r {
        cita_vm::evm::InterpreterResult::Normal(output, _, _) => {
            assert_eq!(
                output,
                vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42]
            );
        }
        _ => panic!(""),
    }
}
