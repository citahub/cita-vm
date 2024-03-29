use ethereum_types::{Address, H256, U256};
use log::debug;
use std::cell::RefCell;
use std::str::FromStr;
use std::sync::Arc;

#[test]
fn test_solidity_simplestorage() {
    let _ = env_logger::builder().is_test(true).try_init();
    let db = Arc::new(cita_vm::state::MemoryDB::new(false));
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
        &Address::from_str("0xBd770416a3345F91E4B34576cb804a576fa48EB1").unwrap(),
        U256::from(10),
        U256::from(1),
        hex::decode(code).unwrap(),
    );
    state.new_contract(
        &Address::from_str("0x1000000000000000000000000000000000000000").unwrap(),
        U256::from(1_000_000_000_000_000u64),
        U256::from(1),
        vec![],
    );

    let block_data_provider: Arc<dyn cita_vm::BlockDataProvider> = Arc::new(cita_vm::BlockDataProviderMock::default());
    let state_data_provider = Arc::new(RefCell::new(state));
    let context = cita_vm::evm::Context::default();
    let config = cita_vm::Config::default();

    // SimpleStorage.set(42)
    let tx = cita_vm::Transaction {
        from: Address::from_str("0x1000000000000000000000000000000000000000").unwrap(),
        to: Some(Address::from_str("0xBd770416a3345F91E4B34576cb804a576fa48EB1").unwrap()),
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
        from: Address::from_str("0x1000000000000000000000000000000000000000").unwrap(),
        to: Some(Address::from_str("0xBd770416a3345F91E4B34576cb804a576fa48EB1").unwrap()),
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
        from: Address::from_str("0x1000000000000000000000000000000000000001").unwrap(), // Omited in most cases.
        to: Some(Address::from_str("0xBd770416a3345F91E4B34576cb804a576fa48EB1").unwrap()), // tx.to shouldn't be none.
        value: U256::from(0),       // tx.value must be 0. This is due to solidity's check.
        nonce: U256::from(123_456), // tx.nonce is just omited.
        gas_limit: 80000,           // Give me a large enougth value plz.
        gas_price: U256::from(1),   // Omited due to solidity's check.
        input: hex::decode("6d4ce63c").unwrap(),
    };
    let r = cita_vm::exec_static(block_data_provider.clone(), state_data_provider, context, config, tx).unwrap();
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

#[test]
fn test_solidity_erc20() {
    let _ = env_logger::builder().is_test(true).try_init();
    let db = Arc::new(cita_vm::state::MemoryDB::new(false));
    let mut state = cita_vm::state::State::new(db).unwrap();
    let address0 = Address::from_str("0x1000000000000000000000000000000000000000").unwrap();
    let address1 = Address::from_str("0x1000000000000000000000000000000000000001").unwrap();

    state.new_contract(&address0, U256::from(100_000_000_000_000_000u64), U256::from(1), vec![]);
    state.new_contract(&address1, U256::from(100_000_000_000_000_000u64), U256::from(1), vec![]);

    // Create a new contract
    let code = "606060405234620000005760405162001617380380620016178339810160405280805190602001909190805182\
                01919060200180519060200190919080518201919050505b83600560003373ffffffffffffffffffffffffffff\
                ffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000208190\
                555083600381905550826000908051906020019082805460018160011615610100020316600290049060005260\
                2060002090601f016020900481019282601f10620000dd57805160ff19168380011785556200010e565b828001\
                600101855582156200010e579182015b828111156200010d578251825591602001919060010190620000f0565b\
                5b5090506200013691905b808211156200013257600081600090555060010162000118565b5090565b50508060\
                019080519060200190828054600181600116156101000203166002900490600052602060002090601f01602090\
                0481019282601f106200018657805160ff1916838001178555620001b7565b82800160010185558215620001b7\
                579182015b82811115620001b657825182559160200191906001019062000199565b5b509050620001df91905b\
                80821115620001db576000816000905550600101620001c1565b5090565b505081600260006101000a81548160\
                ff021916908360ff16021790555033600460006101000a81548173ffffffffffffffffffffffffffffffffffff\
                ffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055505b505050505b6113c48062\
                0002536000396000f300606060405236156100ce576000357c0100000000000000000000000000000000000000\
                000000000000000000900463ffffffff16806306fdde03146100d7578063095ea7b31461016d57806318160ddd\
                146101c157806323b872dd146101e4578063313ce5671461025757806342966c68146102805780636623fc4614\
                6102b557806370a08231146102ea5780638da5cb5b1461033157806395d89b4114610380578063a9059cbb1461\
                0416578063cd4217c114610452578063d7a78db814610499578063dd62ed3e146104ce575b6100d55b5b565b00\
                5b34610000576100e4610534565b60405180806020018281038252838181518152602001915080519060200190\
                80838360008314610133575b805182526020831115610133576020820191506020810190506020830392506101\
                0f565b505050905090810190601f16801561015f5780820380516001836020036101000a031916815260200191\
                505b509250505060405180910390f35b34610000576101a7600480803573ffffffffffffffffffffffffffffff\
                ffffffffff169060200190919080359060200190919050506105d2565b60405180821515151581526020019150\
                5060405180910390f35b34610000576101ce61066f565b6040518082815260200191505060405180910390f35b\
                346100005761023d600480803573ffffffffffffffffffffffffffffffffffffffff1690602001909190803573\
                ffffffffffffffffffffffffffffffffffffffff16906020019091908035906020019091905050610675565b60\
                4051808215151515815260200191505060405180910390f35b3461000057610264610a9b565b604051808260ff\
                1660ff16815260200191505060405180910390f35b346100005761029b6004808035906020019091905050610a\
                ae565b604051808215151515815260200191505060405180910390f35b34610000576102d06004808035906020\
                019091905050610c01565b604051808215151515815260200191505060405180910390f35b346100005761031b\
                600480803573ffffffffffffffffffffffffffffffffffffffff16906020019091905050610dce565b60405180\
                82815260200191505060405180910390f35b346100005761033e610de6565b604051808273ffffffffffffffff\
                ffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019150506040\
                5180910390f35b346100005761038d610e0c565b60405180806020018281038252838181518152602001915080\
                519060200190808383600083146103dc575b8051825260208311156103dc576020820191506020810190506020\
                830392506103b8565b505050905090810190601f1680156104085780820380516001836020036101000a031916\
                815260200191505b509250505060405180910390f35b3461000057610450600480803573ffffffffffffffffff\
                ffffffffffffffffffffff16906020019091908035906020019091905050610eaa565b005b3461000057610483\
                600480803573ffffffffffffffffffffffffffffffffffffffff16906020019091905050611138565b60405180\
                82815260200191505060405180910390f35b34610000576104b46004808035906020019091905050611150565b\
                604051808215151515815260200191505060405180910390f35b346100005761051e600480803573ffffffffff\
                ffffffffffffffffffffffffffffff1690602001909190803573ffffffffffffffffffffffffffffffffffffff\
                ff1690602001909190505061131d565b6040518082815260200191505060405180910390f35b60008054600181\
                600116156101000203166002900480601f01602080910402602001604051908101604052809291908181526020\
                01828054600181600116156101000203166002900480156105ca5780601f1061059f5761010080835404028352\
                91602001916105ca565b820191906000526020600020905b8154815290600101906020018083116105ad578290\
                03601f168201915b505050505081565b60006000821115156105e357610000565b81600760003373ffffffffff\
                ffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081\
                5260200160002060008573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffff\
                ffffffffffffffff16815260200190815260200160002081905550600190505b92915050565b60035481565b60\
                0060008373ffffffffffffffffffffffffffffffffffffffff16141561069b57610000565b6000821115156106\
                aa57610000565b81600560008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffff\
                ffffffffffffffffffffff1681526020019081526020016000205410156106f657610000565b600560008473ff\
                ffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260\
                20019081526020016000205482600560008673ffffffffffffffffffffffffffffffffffffffff1673ffffffff\
                ffffffffffffffffffffffffffffffff1681526020019081526020016000205401101561078357610000565b60\
                0760008573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffff\
                ffff16815260200190815260200160002060003373ffffffffffffffffffffffffffffffffffffffff1673ffff\
                ffffffffffffffffffffffffffffffffffff1681526020019081526020016000205482111561080c5761000056\
                5b610855600560008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffff\
                ffffffffffffff1681526020019081526020016000205483611342565b600560008673ffffffffffffffffffff\
                ffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160\
                0020819055506108e1600560008573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffff\
                ffffffffffffffffffffffff168152602001908152602001600020548361135c565b600560008573ffffffffff\
                ffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081\
                52602001600020819055506109aa600760008673ffffffffffffffffffffffffffffffffffffffff1673ffffff\
                ffffffffffffffffffffffffffffffffff16815260200190815260200160002060003373ffffffffffffffffff\
                ffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001\
                6000205483611342565b600760008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffff\
                ffffffffffffffffffffffffff16815260200190815260200160002060003373ffffffffffffffffffffffffff\
                ffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002081\
                9055508273ffffffffffffffffffffffffffffffffffffffff168473ffffffffffffffffffffffffffffffffff\
                ffffff167fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef8460405180828152\
                60200191505060405180910390a3600190505b9392505050565b600260009054906101000a900460ff1681565b\
                600081600560003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffff\
                ffffffffffff168152602001908152602001600020541015610afc57610000565b600082111515610b0b576100\
                00565b610b54600560003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffff\
                ffffffffffffffffff1681526020019081526020016000205483611342565b600560003373ffffffffffffffff\
                ffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020\
                0160002081905550610ba360035483611342565b6003819055503373ffffffffffffffffffffffffffffffffff\
                ffffff167fcc16f5dbb4873280815c1ee09dbd06736cffcc184412cf7a71a0fdb75d397ca58360405180828152\
                60200191505060405180910390a2600190505b919050565b600081600660003373ffffffffffffffffffffffff\
                ffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020\
                541015610c4f57610000565b600082111515610c5e57610000565b610ca7600660003373ffffffffffffffffff\
                ffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001\
                6000205483611342565b600660003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffff\
                ffffffffffffffffffffffffff16815260200190815260200160002081905550610d33600560003373ffffffff\
                ffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190\
                8152602001600020548361135c565b600560003373ffffffffffffffffffffffffffffffffffffffff1673ffff\
                ffffffffffffffffffffffffffffffffffff168152602001908152602001600020819055503373ffffffffffff\
                ffffffffffffffffffffffffffff167f2cfce4af01bcb9d6cf6c84ee1b7c491100b8695368264146a94d71e10a\
                63083f836040518082815260200191505060405180910390a2600190505b919050565b60056020528060005260\
                406000206000915090505481565b600460009054906101000a900473ffffffffffffffffffffffffffffffffff\
                ffffff1681565b60018054600181600116156101000203166002900480601f0160208091040260200160405190\
                81016040528092919081815260200182805460018160011615610100020316600290048015610ea25780601f10\
                610e7757610100808354040283529160200191610ea2565b820191906000526020600020905b81548152906001\
                0190602001808311610e8557829003601f168201915b505050505081565b60008273ffffffffffffffffffffff\
                ffffffffffffffffff161415610ece57610000565b600081111515610edd57610000565b80600560003373ffff\
                ffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020\
                01908152602001600020541015610f2957610000565b600560008373ffffffffffffffffffffffffffffffffff\
                ffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000205481600560\
                008573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff\
                16815260200190815260200160002054011015610fb657610000565b610fff600560003373ffffffffffffffff\
                ffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020\
                016000205482611342565b600560003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffff\
                ffffffffffffffffffffffffffff1681526020019081526020016000208190555061108b600560008473ffffff\
                ffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001\
                908152602001600020548261135c565b600560008473ffffffffffffffffffffffffffffffffffffffff1673ff\
                ffffffffffffffffffffffffffffffffffffff168152602001908152602001600020819055508173ffffffffff\
                ffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff167fddf252ad1b\
                e2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef836040518082815260200191505060405180\
                910390a35b5050565b60066020528060005260406000206000915090505481565b600081600560003373ffffff\
                ffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001\
                90815260200160002054101561119e57610000565b6000821115156111ad57610000565b6111f6600560003373\
                ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152\
                6020019081526020016000205483611342565b600560003373ffffffffffffffffffffffffffffffffffffffff\
                1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000208190555061128260\
                0660003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffff\
                ffff168152602001908152602001600020548361135c565b600660003373ffffffffffffffffffffffffffffff\
                ffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020819055\
                503373ffffffffffffffffffffffffffffffffffffffff167ff97a274face0b5517365ad396b1fdba6f68bd313\
                5ef603e44272adba3af5a1e0836040518082815260200191505060405180910390a2600190505b919050565b60\
                07602052816000526040600020602052806000526040600020600091509150505481565b600061135083831115\
                611388565b81830390505b92915050565b60006000828401905061137d8482101580156113785750838210155b\
                611388565b8091505b5092915050565b80151561139457610000565b5b505600a165627a7a72305820409669e0\
                0e8d4fc152b0714293e1b1c74ca1dbd321f5137c5b5dc51d29f341460029000000000000000000000000000000\
                000000000000000000000000000000006400000000000000000000000000000000000000000000000000000000\
                000000800000000000000000000000000000000000000000000000000000000000000012000000000000000000\
                00000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000\
                00000000000000000003424e420000000000000000000000000000000000000000000000000000000000000000\
                0000000000000000000000000000000000000000000000000000000003424e4200000000000000000000000000\
                00000000000000000000000000000000";
    let block_data_provider: Arc<dyn cita_vm::BlockDataProvider> = Arc::new(cita_vm::BlockDataProviderMock::default());
    let state_data_provider = Arc::new(RefCell::new(state));
    let context = cita_vm::evm::Context::default();
    let config = cita_vm::Config::default();

    let tx = cita_vm::Transaction {
        from: Address::from_str("0x1000000000000000000000000000000000000000").unwrap(),
        to: None,
        value: U256::from(0),
        nonce: U256::from(1),
        gas_limit: 8_000_000,
        gas_price: U256::from(1),
        input: hex::decode(code).unwrap(),
    };
    let r = cita_vm::exec(
        block_data_provider.clone(),
        state_data_provider.clone(),
        context.clone(),
        config.clone(),
        tx,
    )
    .unwrap();
    let contract = match r {
        cita_vm::evm::InterpreterResult::Create(_, _, _, address) => address,
        _ => panic!("error"),
    };
    println!("{:?}", contract);

    // Call balanceOf
    let tx = cita_vm::Transaction {
        from: address0,
        to: Some(contract),
        value: U256::from(0),
        nonce: U256::from(2),
        gas_limit: 80000,
        gas_price: U256::from(1),
        input: hex::decode("70a082310000000000000000000000001000000000000000000000000000000000000000").unwrap(),
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
        cita_vm::evm::InterpreterResult::Normal(output, _, _) => assert_eq!(
            output,
            vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100]
        ),
        _ => panic!("error"),
    };

    // Transfer value
    let tx = cita_vm::Transaction {
        from: address0,
        to: Some(contract),
        value: U256::from(0),
        nonce: U256::from(3),
        gas_limit: 80000,
        gas_price: U256::from(1),
        input: hex::decode(
"a9059cbb0000000000000000000000001000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000a",
        )
        .unwrap(),
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
        cita_vm::evm::InterpreterResult::Normal(_, _, logs) => {
            assert_eq!(logs.len(), 1);
            let cita_vm::evm::Log(addr, topics, data) = &logs[0];
            assert_eq!(addr, &contract);
            assert_eq!(
                topics[1],
                H256::from_str("0x0000000000000000000000001000000000000000000000000000000000000000").unwrap()
            );
            assert_eq!(
                topics[2],
                H256::from_str("0x0000000000000000000000001000000000000000000000000000000000000001").unwrap()
            );
            assert_eq!(
                data,
                &vec![
                    0u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10
                ]
            )
        }
        _ => panic!("error"),
    };

    // Call balanceOf
    let tx = cita_vm::Transaction {
        from: address0,
        to: Some(contract),
        value: U256::from(0),
        nonce: U256::from(4),
        gas_limit: 80000,
        gas_price: U256::from(1),
        input: hex::decode("70a082310000000000000000000000001000000000000000000000000000000000000000").unwrap(),
    };
    let r = cita_vm::exec(block_data_provider.clone(), state_data_provider, context, config, tx).unwrap();
    match r {
        cita_vm::evm::InterpreterResult::Normal(output, _, _) => assert_eq!(
            output,
            vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 90]
        ),
        _ => panic!("error"),
    };
}
