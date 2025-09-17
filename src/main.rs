mod zeth_trie;
mod hash_builder;

use std::sync::{Arc};
use alloy_consensus::BlockHeader;
use reth_stateless::{stateless_validation_with_trie};
use guest_libs::senders::recover_block;
use {
    reth_chainspec::{ ChainSpec, ChainSpecBuilder},
    reth_stateless::{validation::stateless_validation, ExecutionWitness},
    alloy_primitives::{Bytes, b256, b64, Address, Bloom, Signature, TxKind, U256},
    alloy_consensus::{BlockBody, EthereumTxEnvelope, SignableTransaction},
    reth_evm_ethereum::EthEvmConfig,
    alloy_eips::{eip4895::Withdrawals},

    alloy_primitives::hex,
};
use stateless_trie_bench::{build_execution_witness, get_state_root};

fn main() {
    let witness = build_execution_witness();

    let header = alloy_consensus::Header
    {
        parent_hash: b256!("0xdf1631374b088618d9e4c35250a984da273527f233ec99bbd88e703cb92d402b"),
        ommers_hash: b256!("0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347"),
        beneficiary: Address::from(hex!("0x2adc25665018aa1fe0e6bc666dac8fc2697ff9ba")),
        state_root: b256!("0xec857cddd45a30f8b536eac189b1bc9f556e60072b495404f37eabe89380b712"),
        transactions_root: b256!("0x2e48feb9ef307e371ed9ffc1b2be56ddd7efd32ddf91ec1997765e9485d05f6b"),
        receipts_root: b256!("0xe1041cfa2ab2edcfb376facb8629be36dc139550978ce2dc726e436a2d766586"),
        withdrawals_root: Some(b256!("0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421")),
        logs_bloom: Bloom::from(hex!("0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")),
        difficulty: U256::from(0),
        number: 1,
        gas_limit: 36000000,
        gas_used: 90111,
        timestamp: 1000,
        mix_hash: b256!("0x0000000000000000000000000000000000000000000000000000000000000000"),
        nonce: b64!("0x0000000000000000"),
        base_fee_per_gas: Some(7),
        blob_gas_used: Some(0),
        excess_blob_gas: Some(0),
        parent_beacon_block_root: Some(b256!("0x0000000000000000000000000000000000000000000000000000000000000000")),
        requests_hash: Some(b256!("0xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")),
        extra_data: Bytes::from(hex!("0x00"))
    };

    let tx = alloy_consensus::TxLegacy
    {
        chain_id: Some(1),
        nonce: 0,
        gas_price: 10,
        gas_limit: 126809,
        to: TxKind::Call(Address::from(hex!("0x0000000000000000000000000000000000001000"))),
        value: U256::from(0),
        input: Bytes::from(hex!("0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000017f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb0000000000000000000000000000000008b3f481e3aaa0f1a09e30ed741d8ae4fcf5e095d5d00af600db18cb2c04b3edd03cc744a2888ae40caa232946c5e7e1"))
    };

    let stx = tx.into_signed(Signature::new(
        b256!("0x71c4335e62a71ef3cb6f818eaf1fd3b9e86e8ac1ab7f24ac7e865d9b190c4f1e").into(),
        b256!("0x4ec2a4c738a770acb10e4afdb5bf011dc2b5d58be76c76ddc544bca92c8280c4").into(),
        true,
    ));


    let envelope = EthereumTxEnvelope::from(stx);

    let bb = BlockBody::<_, alloy_consensus::Header>
    {
        transactions: [envelope].to_vec(),
        ommers: [].to_vec(),
        withdrawals: Some(Withdrawals::default())
    };

    let b = alloy_consensus::Block::new(header, bb);
    let chain_spec: Arc<ChainSpec> = ChainSpecBuilder::mainnet().prague_activated().build().into();
    let recovered_block = recover_block(b, chain_spec.as_ref()).unwrap();

    let evm_config = EthEvmConfig::new(chain_spec.clone());

    use std::time::Instant;
    let mut now = Instant::now();

    let r = stateless_validation(
        recovered_block.clone(),
        witness.clone(),
        chain_spec.clone(),
        evm_config.clone()
    );

    println!("{:?}", now.elapsed());

    if r.is_err() {
        panic!("Error")
    }

    now = Instant::now();
    let r1 =
        stateless_validation_with_trie::<zeth_trie::SparseState, ChainSpec, EthEvmConfig>(
            recovered_block.clone(),
            witness.clone(),
            chain_spec.clone(),
            evm_config.clone()
        );
    println!("{:?}", now.elapsed());
    if r1.is_err() {
        panic!("Error")
    }
}
