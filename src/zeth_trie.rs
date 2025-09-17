use alloy_primitives::{Address, B256, Bytes, KECCAK256_EMPTY, U256, keccak256, map::B256Map};
use alloy_trie::{EMPTY_ROOT_HASH, TrieAccount};
use reth_errors::ProviderError;
use reth_stateless::validation::StatelessValidationError;
use reth_trie_common::HashedPostState;
use revm_bytecode::Bytecode;
use risc0_ethereum_trie::CachedTrie;
use std::{cell::RefCell, collections::hash_map::Entry, fmt::Debug, marker::PhantomData};

pub use reth_stateless::{ExecutionWitness, StatelessTrie};

/// Zero-overhead helper for tries that only contain RLP encoded data.
#[derive(Debug, Clone, Default)]
#[repr(transparent)]
struct RlpTrie<T> {
    inner: CachedTrie,
    phantom: PhantomData<T>,
}

impl<T: alloy_rlp::Decodable + alloy_rlp::Encodable> RlpTrie<T> {
    fn new(inner: CachedTrie) -> Self {
        Self { inner, phantom: PhantomData }
    }

    pub fn from_prehashed(
        root: B256,
        rlp_by_digest: &B256Map<impl AsRef<[u8]>>,
    ) -> alloy_rlp::Result<Self> {
        Ok(Self::new(CachedTrie::from_prehashed_nodes(root, rlp_by_digest)?))
    }

    pub fn get(&self, key: impl AsRef<[u8]>) -> alloy_rlp::Result<Option<T>> {
        self.inner.get(key).map(alloy_rlp::decode_exact).transpose()
    }

    pub fn insert(&mut self, key: impl AsRef<[u8]>, value: T) {
        self.inner.insert(key, alloy_rlp::encode(value));
    }

    pub fn remove(&mut self, key: impl AsRef<[u8]>) -> bool {
        self.inner.remove(key)
    }

    pub fn hash(&mut self) -> B256 {
        self.inner.hash()
    }
}

/// Represents a sparse version of the Ethereum world state.
/// This is significantly more performant than the Reth default.
#[derive(Debug, Clone)]
pub struct SparseState {
    /// state MPT containing all used accounts
    state: RlpTrie<TrieAccount>,
    /// storage MPTs sorted by the hashed address of their account
    storages: RefCell<B256Map<RlpTrie<U256>>>,

    /// all relevant MPT nodes by their Keccak hash
    rlp_by_digest: B256Map<Bytes>,
}

impl SparseState {
    /// Removes an account from the state.
    fn remove_account(&mut self, hashed_address: &B256) {
        self.state.remove(hashed_address);
        self.storages.get_mut().remove(hashed_address);
    }

    /// Clears the storage of an account.
    fn clear_storage(&mut self, hashed_address: B256) -> &mut RlpTrie<U256> {
        self.storages.get_mut().entry(hashed_address).insert_entry(RlpTrie::default()).into_mut()
    }

    /// Returns a mutable version of the storage trie of the given account.
    fn storage_trie_mut(&mut self, hashed_address: B256) -> alloy_rlp::Result<&mut RlpTrie<U256>> {
        let trie = match self.storages.get_mut().entry(hashed_address) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                // build the storage trie matching the storage root of the account
                let storage_root =
                    self.state.get(hashed_address)?.map_or(EMPTY_ROOT_HASH, |a| a.storage_root);
                entry.insert(RlpTrie::from_prehashed(storage_root, &self.rlp_by_digest)?)
            }
        };

        Ok(trie)
    }
}

impl StatelessTrie for SparseState {
    /// Initialize the stateless trie using the `ExecutionWitness`.
    fn new(
        witness: &ExecutionWitness,
        pre_state_root: B256,
    ) -> Result<(Self, B256Map<Bytecode>), StatelessValidationError> {
        // fist, hash all the RLP nodes once
        let rlp_by_digest: B256Map<_> =
            witness.state.iter().map(|rlp| (keccak256(rlp), rlp.clone())).collect();

        // construct the state trie from the witness data and the given state root
        let state = RlpTrie::from_prehashed(pre_state_root, &rlp_by_digest)
            .map_err(|_| StatelessValidationError::WitnessRevealFailed { pre_state_root })?;

        // hash all the supplied bytecode
        let bytecode = witness
            .codes
            .iter()
            .map(|code| (keccak256(code), Bytecode::new_raw(code.clone())))
            .collect();

        Ok((Self { state, storages: RefCell::new(B256Map::default()), rlp_by_digest }, bytecode))
    }

    /// Returns the `TrieAccount` that corresponds to the `Address`.
    fn account(&self, address: Address) -> Result<Option<TrieAccount>, ProviderError> {
        let hashed_address = keccak256(address);
        match self.state.get(hashed_address)? {
            None => Ok(None),
            Some(account) => {
                // each time an account is accessed, check whether its storage trie already exists
                // otherwise construct it from the witness data and the account's storage root
                match self.storages.borrow_mut().entry(hashed_address) {
                    Entry::Vacant(entry) => {
                        entry.insert(RlpTrie::from_prehashed(
                            account.storage_root,
                            &self.rlp_by_digest,
                        )?);
                    }
                    Entry::Occupied(_) => {}
                }

                Ok(Some(account))
            }
        }
    }

    /// Returns the storage slot value that corresponds to the given (address, slot) tuple.
    fn storage(&self, address: Address, slot: U256) -> Result<U256, ProviderError> {
        let storages = self.storages.borrow();
        // storage() is always be called after account(), so the storage trie must already exist
        let storage_trie = storages.get(&keccak256(address)).unwrap();
        Ok(storage_trie.get(keccak256(B256::from(slot)))?.unwrap_or(U256::ZERO))
    }

    /// Computes the new state root from the HashedPostState.
    fn calculate_state_root(
        &mut self,
        state: HashedPostState,
    ) -> Result<B256, StatelessValidationError> {
        let mut removed_accounts = Vec::new();
        for (hashed_address, account) in state.accounts {
            // nonexisting accounts must be removed from the state
            let Some(account) = account else {
                removed_accounts.push(hashed_address);
                continue;
            };

            // apply storage changes before computing the storage root
            let storage_root = match state.storages.get(&hashed_address) {
                None => self.storage_trie_mut(hashed_address).unwrap().hash(),
                Some(storage) => {
                    let storage_trie = if storage.wiped {
                        self.clear_storage(hashed_address)
                    } else {
                        self.storage_trie_mut(hashed_address).unwrap()
                    };

                    // apply all state modifications
                    for (hashed_key, value) in &storage.storage {
                        if !value.is_zero() {
                            storage_trie.insert(hashed_key, *value);
                        }
                    }
                    // removals must happen last, otherwise unresolved orphans might still exist
                    for (hashed_key, value) in &storage.storage {
                        if value.is_zero() {
                            storage_trie.remove(hashed_key);
                        }
                    }

                    storage_trie.hash()
                }
            };

            // update/insert the account after all changes have been processed
            let account = TrieAccount {
                nonce: account.nonce,
                balance: account.balance,
                storage_root,
                code_hash: account.bytecode_hash.unwrap_or(KECCAK256_EMPTY),
            };
            self.state.insert(hashed_address, account);
        }
        removed_accounts.iter().for_each(|hashed_address| self.remove_account(hashed_address));

        Ok(self.state.hash())
    }
}