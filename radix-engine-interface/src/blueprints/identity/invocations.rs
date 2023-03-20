use crate::blueprints::resource::*;
use crate::*;
use radix_engine_common::data::scrypto::model::ComponentAddress;
use sbor::rust::fmt::Debug;

pub const IDENTITY_BLUEPRINT: &str = "Identity";

pub const IDENTITY_CREATE_ADVANCED_IDENT: &str = "create_advanced";

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor, ManifestSbor)]
pub struct IdentityCreateAdvancedInput {
    pub config: AccessRulesConfig,
}

pub type IdentityCreateAdvancedOutput = ComponentAddress;

pub const IDENTITY_CREATE_IDENT: &str = "create";

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor, ManifestSbor)]
pub struct IdentityCreateInput {}

pub type IdentityCreateOutput = (ComponentAddress, Bucket);

pub const IDENTITY_SECURIFY_IDENT: &str = "securify";

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor, ManifestSbor)]
pub struct IdentitySecurifyToSingleBadgeInput {}

pub type IdentitySecurifyToSingleBadgeOutput = Bucket;
