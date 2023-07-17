use scrypto::prelude::*;

#[blueprint]
mod function_access_rules {
    enable_function_auth! {
        protected_function => rule!(require(RADIX_TOKEN));
        public_function => rule!(allow_all);
    }

    struct FunctionAccessRules {}

    impl FunctionAccessRules {
        pub fn protected_function() {}

        pub fn public_function() {}
    }
}
