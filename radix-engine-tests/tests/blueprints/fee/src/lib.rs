use scrypto::prelude::*;

#[blueprint]
mod fee {
    struct Fee {
        xrd: Vault,
        xrd_empty: Vault,
        doge: Vault,
        garbage_vaults: Vec<Vault>,
    }

    impl Fee {
        pub fn new(xrd: Bucket) -> Global<Fee> {
            let doge_tokens = ResourceBuilder::new_fungible(OwnerRole::None)
                .metadata(metadata! {
                    init {
                        "name" => "DogeCoin".to_owned(), locked;
                    }
                })
                .mint_initial_supply(100);

            Self {
                xrd: Vault::with_bucket(xrd),
                xrd_empty: Vault::new(RADIX_TOKEN),
                doge: Vault::with_bucket(doge_tokens),
                garbage_vaults: Vec::new(),
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::None)
            .globalize()
        }

        pub fn lock_fee(&mut self, amount: Decimal) {
            self.xrd.as_fungible().lock_fee(amount);
        }

        pub fn lock_fee_with_empty_vault(&mut self, amount: Decimal) {
            self.xrd_empty.as_fungible().lock_fee(amount);
        }

        pub fn lock_fee_with_doge(&mut self, amount: Decimal) {
            self.doge.as_fungible().lock_fee(amount);
        }

        pub fn lock_fee_with_temp_vault(&mut self, amount: Decimal) {
            let vault = Vault::new(RADIX_TOKEN);
            vault.as_fungible().lock_fee(amount);
            self.garbage_vaults.push(vault);
        }

        pub fn update_vault_and_lock_fee(&mut self, amount: Decimal) {
            info!("Balance: {}", self.xrd.amount());
            let bucket = self.xrd.take(Decimal::from(1u32));
            self.xrd.put(bucket);
            self.xrd.as_fungible().lock_fee(amount);
        }

        pub fn query_vault_and_lock_fee(&mut self, amount: Decimal) {
            info!("Balance: {}", self.xrd.amount());
            self.xrd.as_fungible().lock_fee(amount);
        }

        pub fn lock_fee_and_query_vault(&mut self, amount: Decimal) {
            self.xrd.as_fungible().lock_fee(amount);
            info!("Balance: {}", self.xrd.amount());
        }

        pub fn spin_loop(&self) {
            let mut n: u64 = 0;
            loop {
                n += 1;
                // Avoid loop being optimised away!
                std::hint::black_box(n);
            }
        }
    }
}
