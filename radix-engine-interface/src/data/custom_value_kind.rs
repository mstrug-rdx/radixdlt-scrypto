use sbor::*;

pub const VALUE_KIND_PACKAGE_ADDRESS: u8 = 0x80;
pub const VALUE_KIND_COMPONENT_ADDRESS: u8 = 0x81;
pub const VALUE_KIND_RESOURCE_ADDRESS: u8 = 0x82;
pub const VALUE_KIND_SYSTEM_ADDRESS: u8 = 0x83;

pub const VALUE_KIND_OWN: u8 = 0x94;
pub const VALUE_KIND_NON_FUNGIBLE_ADDRESS: u8 = 0xa2;
pub const VALUE_KIND_BLOB: u8 = 0xa1;

pub const VALUE_KIND_BUCKET: u8 = 0x92;
pub const VALUE_KIND_PROOF: u8 = 0x93;
pub const VALUE_KIND_EXPRESSION: u8 = 0xa0;

pub const VALUE_KIND_HASH: u8 = 0xb0;
pub const VALUE_KIND_ECDSA_SECP256K1_PUBLIC_KEY: u8 = 0xb1;
pub const VALUE_KIND_ECDSA_SECP256K1_SIGNATURE: u8 = 0xb2;
pub const VALUE_KIND_EDDSA_ED25519_PUBLIC_KEY: u8 = 0xb3;
pub const VALUE_KIND_EDDSA_ED25519_SIGNATURE: u8 = 0xb4;
pub const VALUE_KIND_DECIMAL: u8 = 0xb5;
pub const VALUE_KIND_PRECISE_DECIMAL: u8 = 0xb6;
pub const VALUE_KIND_NON_FUNGIBLE_ID: u8 = 0xb7;

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "type")
)]
#[derive(Copy, Debug, Clone, PartialEq, Eq)]
pub enum ScryptoCustomValueKind {
    // RE global address types
    PackageAddress,
    ComponentAddress,
    ResourceAddress,
    SystemAddress,

    // RE interpreted types
    Own,
    NonFungibleAddress, // for resource address contained
    Blob,

    // TX interpreted types (TODO: rename?)
    Bucket,     // super::types::ManifestBucket
    Proof,      // super::types::ManifestProof
    Expression, // super::types::ManifestExpression

    // Uninterpreted
    Hash,
    EcdsaSecp256k1PublicKey,
    EcdsaSecp256k1Signature,
    EddsaEd25519PublicKey,
    EddsaEd25519Signature,
    Decimal,
    PreciseDecimal,
    NonFungibleId,
}

impl From<ScryptoCustomValueKind> for ValueKind<ScryptoCustomValueKind> {
    fn from(custom_value_kind: ScryptoCustomValueKind) -> Self {
        ValueKind::Custom(custom_value_kind)
    }
}

impl CustomValueKind for ScryptoCustomValueKind {
    fn as_u8(&self) -> u8 {
        match self {
            Self::PackageAddress => VALUE_KIND_PACKAGE_ADDRESS,
            Self::ComponentAddress => VALUE_KIND_COMPONENT_ADDRESS,
            Self::ResourceAddress => VALUE_KIND_RESOURCE_ADDRESS,
            Self::SystemAddress => VALUE_KIND_SYSTEM_ADDRESS,
            Self::Own => VALUE_KIND_OWN,
            Self::Bucket => VALUE_KIND_BUCKET,
            Self::Proof => VALUE_KIND_PROOF,
            Self::Expression => VALUE_KIND_EXPRESSION,
            Self::Blob => VALUE_KIND_BLOB,
            Self::NonFungibleAddress => VALUE_KIND_NON_FUNGIBLE_ADDRESS,
            Self::Hash => VALUE_KIND_HASH,
            Self::EcdsaSecp256k1PublicKey => VALUE_KIND_ECDSA_SECP256K1_PUBLIC_KEY,
            Self::EcdsaSecp256k1Signature => VALUE_KIND_ECDSA_SECP256K1_SIGNATURE,
            Self::EddsaEd25519PublicKey => VALUE_KIND_EDDSA_ED25519_PUBLIC_KEY,
            Self::EddsaEd25519Signature => VALUE_KIND_EDDSA_ED25519_SIGNATURE,
            Self::Decimal => VALUE_KIND_DECIMAL,
            Self::PreciseDecimal => VALUE_KIND_PRECISE_DECIMAL,
            Self::NonFungibleId => VALUE_KIND_NON_FUNGIBLE_ID,
        }
    }

    fn from_u8(id: u8) -> Option<Self> {
        match id {
            VALUE_KIND_PACKAGE_ADDRESS => Some(ScryptoCustomValueKind::PackageAddress),
            VALUE_KIND_COMPONENT_ADDRESS => Some(ScryptoCustomValueKind::ComponentAddress),
            VALUE_KIND_RESOURCE_ADDRESS => Some(ScryptoCustomValueKind::ResourceAddress),
            VALUE_KIND_SYSTEM_ADDRESS => Some(ScryptoCustomValueKind::SystemAddress),
            VALUE_KIND_OWN => Some(ScryptoCustomValueKind::Own),
            VALUE_KIND_BUCKET => Some(ScryptoCustomValueKind::Bucket),
            VALUE_KIND_PROOF => Some(ScryptoCustomValueKind::Proof),
            VALUE_KIND_EXPRESSION => Some(ScryptoCustomValueKind::Expression),
            VALUE_KIND_BLOB => Some(ScryptoCustomValueKind::Blob),
            VALUE_KIND_NON_FUNGIBLE_ADDRESS => Some(ScryptoCustomValueKind::NonFungibleAddress),
            VALUE_KIND_HASH => Some(ScryptoCustomValueKind::Hash),
            VALUE_KIND_ECDSA_SECP256K1_PUBLIC_KEY => {
                Some(ScryptoCustomValueKind::EcdsaSecp256k1PublicKey)
            }
            VALUE_KIND_ECDSA_SECP256K1_SIGNATURE => {
                Some(ScryptoCustomValueKind::EcdsaSecp256k1Signature)
            }
            VALUE_KIND_EDDSA_ED25519_PUBLIC_KEY => {
                Some(ScryptoCustomValueKind::EddsaEd25519PublicKey)
            }
            VALUE_KIND_EDDSA_ED25519_SIGNATURE => {
                Some(ScryptoCustomValueKind::EddsaEd25519Signature)
            }
            VALUE_KIND_DECIMAL => Some(ScryptoCustomValueKind::Decimal),
            VALUE_KIND_PRECISE_DECIMAL => Some(ScryptoCustomValueKind::PreciseDecimal),
            VALUE_KIND_NON_FUNGIBLE_ID => Some(ScryptoCustomValueKind::NonFungibleId),
            _ => None,
        }
    }
}
