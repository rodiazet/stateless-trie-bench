mod hash_builder;
use guest_libs::mpt::SparseState;
use guest_libs::senders::recover_block;
use stateless_trie_bench::{get_test_file_path, load_stateless_input};
use std::sync::Arc;
use {
    reth_chainspec::ChainSpec,
    reth_evm_ethereum::EthEvmConfig,
    reth_stateless::{Genesis, stateless_validation_with_trie, validation::stateless_validation},
};

fn main() {
    let input = load_stateless_input(&get_test_file_path());

    let genesis = Genesis {
        config: input.chain_config.clone(),
        ..Default::default()
    };
    let chain_spec: Arc<ChainSpec> = Arc::new(genesis.into());
    let recovered_block = recover_block(input.block, &chain_spec).unwrap();
    let evm_config = EthEvmConfig::new(chain_spec.clone());

    use std::time::Instant;
    let mut now = Instant::now();

    let r = stateless_validation(
        recovered_block.clone(),
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
        recovered_block.clone(),
        input.witness.clone(),
        chain_spec.clone(),
        evm_config.clone(),
    );
    println!("{:?}", now.elapsed());
    if r1.is_err() {
        panic!("Error")
    }
}
