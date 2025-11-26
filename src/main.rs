mod hash_builder;

use std::sync::Arc;
use sparsestate::SparseState;
use stateless_trie_bench::{get_state_root, get_test_file_path, load_execution_witness, load_stateless_input};
use {
    reth_chainspec::ChainSpec,
    reth_evm_ethereum::EthEvmConfig,
    reth_stateless::{Genesis, stateless_validation_with_trie, validation::stateless_validation},
};
use guest_libs::senders::recover_signers;
use anyhow;
use simple_sparse_state;
use std::fmt::Display;
use alloy_primitives::{Address, FixedBytes};
use reth_stateless::StatelessTrie;

static TEST_FILE: &str = "test_data/mainnet_block_164E2F4_test.json";
fn main() {
    let witness = load_execution_witness(&String::from(TEST_FILE));
    let state_root = get_state_root(&witness);
    let trie = simple_sparse_state::SimpleSparseState::new(&witness, state_root).unwrap().0;
    
    let addresses: Vec<Address> = witness
        .keys
        .iter()
        .filter(|key| key.len() == 20)
        .map(|key| Address::from(FixedBytes::<20>::from_slice(key)))
        .collect();
    
    for n in 1..=500 {
        let trie_copy = trie.clone();
        for address in addresses.iter() {
            assert!(trie_copy.account(address.clone()).is_ok());
        }
    }

    let input = load_stateless_input(&get_test_file_path());

    let genesis = Genesis {
        config: input.chain_config.clone(),
        ..Default::default()
    };
    let chain_spec: Arc<ChainSpec> = Arc::new(genesis.into());
    let evm_config = EthEvmConfig::new(chain_spec.clone());

    use std::time::Instant;
    let mut now = Instant::now();

    let public_keys = recover_signers(input.block.body.transactions.iter())
        .map_err(|err| anyhow::anyhow!("recovering signers: {err}")).unwrap();

    let r = stateless_validation(
        input.block.clone(),
        public_keys.clone(),
        input.witness.clone(),
        chain_spec.clone(),
        evm_config.clone(),
    );

    println!("{:?}", now.elapsed());

    if r.is_err() {
        panic!("Error")
    }

    now = Instant::now();
    let r1 = stateless_validation_with_trie::<SparseState, ChainSpec, EthEvmConfig>(
        input.block.clone(),
        public_keys.clone(),
        input.witness.clone(),
        chain_spec.clone(),
        evm_config.clone(),
    );
    println!("{:?}", now.elapsed());
    if r1.is_err() {
        panic!("Error")
    }

    println!("--------");
    now = Instant::now();
    let r2 = stateless_validation_with_trie::<simple_sparse_state::SimpleSparseState, ChainSpec, EthEvmConfig>(
        input.block.clone(),
        public_keys.clone(),
        input.witness.clone(),
        chain_spec.clone(),
        evm_config.clone(),
    );
    if r2.is_err() {
        panic!("Error")
    }
    println!("{:?}", now.elapsed());
}
