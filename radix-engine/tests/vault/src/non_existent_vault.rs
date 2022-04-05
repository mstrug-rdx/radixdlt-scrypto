use scrypto::prelude::*;

blueprint! {
    struct NonExistentVault {
        vault: Option<Vault>,
        vaults: LazyMap<u128, Vault>,
    }

    impl NonExistentVault {
        pub fn create_component_with_non_existent_vault() -> ComponentId {
            NonExistentVault {
                vault: Option::Some(Vault((Transaction::transaction_hash(), 1025))),
                vaults: LazyMap::new(),
            }
            .instantiate()
            .globalize()
        }

        pub fn new() -> ComponentId {
            NonExistentVault {
                vault: Option::None,
                vaults: LazyMap::new(),
            }
            .instantiate()
            .globalize()
        }

        pub fn create_non_existent_vault(&mut self) {
            self.vault = Option::Some(Vault((Transaction::transaction_hash(), 1025)))
        }

        pub fn create_lazy_map_with_non_existent_vault() -> ComponentId {
            let vaults = LazyMap::new();
            vaults.insert(0, Vault((Transaction::transaction_hash(), 1025)));
            NonExistentVault {
                vault: Option::None,
                vaults,
            }
            .instantiate()
            .globalize()
        }

        pub fn create_non_existent_vault_in_lazy_map(&mut self) {
            self.vaults
                .insert(0, Vault((Transaction::transaction_hash(), 1025)));
        }
    }
}
