#![allow(soft_unstable, unused)]

extern crate alloc;
extern crate core;

mod utils;

use crate::utils::EvmTestingContext;
use core::str::from_utf8;
use fluentbase_sdk::{bytes::BytesMut, codec::SolidityABI, Address, Bytes};
use hex_literal::hex;
use sp1_sdk::{include_elf, ProverClient, SP1Stdin};

const SP1_ELF: &[u8] = include_elf!("rwasm-bench-sp1");

fn execute_rwasm_sp1_test(wasm_binary: &[u8], input: &[u8]) -> Bytes {
    // deploy and call greeting WASM contract
    let mut ctx = EvmTestingContext::default();
    const DEPLOYER_ADDRESS: Address = Address::ZERO;
    let contract_address = ctx.deploy_evm_tx(DEPLOYER_ADDRESS, Bytes::copy_from_slice(wasm_binary));
    let result = ctx.call_evm_tx(
        DEPLOYER_ADDRESS,
        contract_address,
        Bytes::copy_from_slice(input),
        None,
        None,
    );
    let output = result.output().cloned().unwrap_or_default();
    println!("Result: {:?}", result);
    assert!(result.is_success());
    // run sp1 test
    let mut stdin = SP1Stdin::new();
    stdin.write_vec(wasm_binary.to_vec());
    let mut aligned_input: Vec<u8> = vec![0u8; 380];
    aligned_input.extend_from_slice(&input);
    stdin.write_vec(aligned_input);
    let client = ProverClient::from_env();
    let (result, report) = client.execute(SP1_ELF, &stdin).run().unwrap();
    println!(
        "RISC-V opcode counts: {:?}",
        report.opcode_counts.as_slice().iter().sum::<u64>()
    );
    // make sure rWasm and SP1 outputs are the same
    assert_eq!(result.as_slice(), output.as_ref());
    output
}

#[test]
fn test_wasm_greeting() {
    let output = execute_rwasm_sp1_test(
        include_bytes!("../assets/greeting.wasm"),
        Default::default(),
    );
    assert_eq!("Hello, World", from_utf8(output.as_ref()).unwrap());
}

#[test]
fn test_wasm_keccak256() {
    let output = execute_rwasm_sp1_test(
        include_bytes!("../assets/keccak256.wasm"),
        "Hello, World".as_bytes(),
    );
    assert_eq!(
        &hex!("a04a451028d0f9284ce82243755e245238ab1e4ecf7b9dd8bf4734d9ecfd0529"),
        output.as_ref(),
    );
}

#[test]
fn test_wasm_secp256k1() {
    let _output = execute_rwasm_sp1_test(
        include_bytes!("../assets/secp256k1.wasm"),
        &hex!("a04a451028d0f9284ce82243755e245238ab1e4ecf7b9dd8bf4734d9ecfd0529cf09dd8d0eb3c3968aca8846a249424e5537d3470f979ff902b57914dc77d02316bd29784f668a73cc7a36f4cc5b9ce704481e6cb5b1c2c832af02ca6837ebec044e3b81af9c2234cad09d679ce6035ed1392347ce64ce405f5dcd36228a25de6e47fd35c4215d1edf53e6f83de344615ce719bdb0fd878f6ed76f06dd277956de"),
    );
}

#[test]
fn test_wasm_checkmate() {
    let mut input = BytesMut::new();
    SolidityABI::<(String, String)>::encode(
        &(
            "rnbq1k1r/1p1p3p/5npb/2pQ1p2/p1B1P2P/8/PPP2PP1/RNB1K1NR w KQ - 2 11".to_string(),
            "Qf7".to_string(),
        ),
        &mut input,
        0,
    )
    .unwrap();
    let input = input.freeze().to_vec();
    let _output = execute_rwasm_sp1_test(include_bytes!("../assets/checkmate.wasm"), &input);
}

#[test]
fn test_wasm_json() {
    let _output = execute_rwasm_sp1_test(
        include_bytes!("../assets/json.wasm"),
        "{\"message\": \"Hello, World\"}".as_bytes(),
    );
}
