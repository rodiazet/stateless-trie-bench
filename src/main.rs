mod hash_builder;

use std::sync::Arc;
use sparsestate::SparseState;
use stateless_trie_bench::{get_test_file_path, load_stateless_input};
use {
    reth_chainspec::ChainSpec,
    reth_evm_ethereum::EthEvmConfig,
    reth_stateless::{Genesis, stateless_validation_with_trie, validation::stateless_validation},
};
use guest_libs::senders::recover_signers;
use anyhow;
use simple_sparse_state;

fn main() {
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

    now = Instant::now();
    let r2 = stateless_validation_with_trie::<simple_sparse_state::SimpleSparseState, ChainSpec, EthEvmConfig>(
        input.block.clone(),
        public_keys.clone(),
        input.witness.clone(),
        chain_spec.clone(),
        evm_config.clone(),
    );
    println!("{:?}", now.elapsed());
    if r2.is_err() {
        panic!("Error")
    }
}
