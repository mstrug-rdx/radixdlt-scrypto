use crate::address::*;
use crate::constants::*;
use crate::crypto::*;
use crate::data::scrypto::model::*;
use crate::types::Blueprint;
use crate::*;
use radix_engine_common::data::scrypto::scrypto_encode;
use radix_engine_common::types::*;
use sbor::rust::fmt;
use sbor::rust::format;
use sbor::rust::str::FromStr;
use sbor::rust::string::String;
use sbor::rust::vec::Vec;
use utils::ContextualDisplay;

/// Represents the global id of a non-fungible.
#[derive(Clone, PartialEq, Eq, Hash, Ord, PartialOrd, ScryptoSbor, ManifestSbor)]
pub struct NonFungibleGlobalId(ResourceAddress, NonFungibleLocalId);

impl NonFungibleGlobalId {
    pub const fn new(resource_address: ResourceAddress, local_id: NonFungibleLocalId) -> Self {
        Self(resource_address, local_id)
    }

    pub fn package_of_caller_badge(address: PackageAddress) -> Self {
        let local_id = NonFungibleLocalId::bytes(scrypto_encode(&address).unwrap()).unwrap();
        NonFungibleGlobalId::new(PACKAGE_OF_CALLER_VIRTUAL_BADGE, local_id)
    }

    pub fn global_caller_badge(global_caller: GlobalCaller) -> Self {
        let local_id = NonFungibleLocalId::bytes(scrypto_encode(&global_caller).unwrap()).unwrap();
        NonFungibleGlobalId::new(GLOBAL_CALLER_VIRTUAL_BADGE, local_id)
    }

    /// Returns the resource address.
    pub fn resource_address(&self) -> ResourceAddress {
        self.0
    }

    /// Returns the non-fungible id.
    pub fn local_id(&self) -> &NonFungibleLocalId {
        &self.1
    }

    /// Returns canonical representation of this NonFungibleGlobalId.
    pub fn to_canonical_string(&self, bech32_encoder: &Bech32Encoder) -> String {
        format!("{}", self.display(bech32_encoder))
    }

    /// Converts canonical representation to NonFungibleGlobalId.
    ///
    /// This is composed of `resource_address:id_simple_representation`
    pub fn try_from_canonical_string(
        bech32_decoder: &Bech32Decoder,
        s: &str,
    ) -> Result<Self, ParseNonFungibleGlobalIdError> {
        let parts = s.split(':').collect::<Vec<&str>>();
        if parts.len() != 2 {
            return Err(ParseNonFungibleGlobalIdError::RequiresTwoParts);
        }
        let resource_address = ResourceAddress::try_from_bech32(bech32_decoder, parts[0])
            .ok_or(ParseNonFungibleGlobalIdError::InvalidResourceAddress)?;
        let local_id = NonFungibleLocalId::from_str(parts[1])?;
        Ok(NonFungibleGlobalId::new(resource_address, local_id))
    }
}

#[derive(Clone, Debug, ScryptoSbor)]
pub enum GlobalCaller {
    /// If the previous global frame started with an object's main module
    GlobalObject(GlobalAddress),
    /// If the previous global frame started with a function call
    PackageBlueprint(PackageAddress, String),
}

impl From<ComponentAddress> for GlobalCaller {
    fn from(value: ComponentAddress) -> Self {
        GlobalCaller::GlobalObject(value.into())
    }
}

impl From<GlobalAddress> for GlobalCaller {
    fn from(value: GlobalAddress) -> Self {
        GlobalCaller::GlobalObject(value)
    }
}

impl From<(PackageAddress, String)> for GlobalCaller {
    fn from((package, blueprint): (PackageAddress, String)) -> Self {
        GlobalCaller::PackageBlueprint(package, blueprint)
    }
}

impl From<Blueprint> for GlobalCaller {
    fn from(blueprint: Blueprint) -> Self {
        GlobalCaller::PackageBlueprint(blueprint.package_address, blueprint.blueprint_name)
    }
}

//======
// error
//======

/// Represents an error when parsing non-fungible address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseNonFungibleGlobalIdError {
    InvalidResourceAddress,
    InvalidNonFungibleLocalId(ParseNonFungibleLocalIdError),
    RequiresTwoParts,
}

impl From<ParseNonFungibleLocalIdError> for ParseNonFungibleGlobalIdError {
    fn from(err: ParseNonFungibleLocalIdError) -> Self {
        Self::InvalidNonFungibleLocalId(err)
    }
}

#[cfg(not(feature = "alloc"))]
impl std::error::Error for ParseNonFungibleGlobalIdError {}

#[cfg(not(feature = "alloc"))]
impl fmt::Display for ParseNonFungibleGlobalIdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

//======
// text
//======

impl<'a> ContextualDisplay<AddressDisplayContext<'a>> for NonFungibleGlobalId {
    type Error = fmt::Error;

    fn contextual_format<F: fmt::Write>(
        &self,
        f: &mut F,
        context: &AddressDisplayContext<'a>,
    ) -> Result<(), Self::Error> {
        write!(
            f,
            "{}:{}",
            self.resource_address().display(*context),
            self.local_id()
        )
    }
}

impl fmt::Debug for NonFungibleGlobalId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.display(NO_NETWORK))
    }
}

pub trait FromPublicKey: Sized {
    fn from_public_key<P: Into<PublicKey> + Clone>(public_key: &P) -> Self;
}

impl FromPublicKey for NonFungibleGlobalId {
    fn from_public_key<P: Into<PublicKey> + Clone>(public_key: &P) -> Self {
        let public_key: PublicKey = public_key.clone().into();

        match public_key {
            PublicKey::EcdsaSecp256k1(public_key) => {
                let id: [u8; NodeId::UUID_LENGTH] = hash(public_key.to_vec()).lower_bytes();
                NonFungibleGlobalId::new(
                    ECDSA_SECP256K1_SIGNATURE_VIRTUAL_BADGE,
                    NonFungibleLocalId::bytes(id.to_vec()).unwrap(),
                )
            }
            PublicKey::EddsaEd25519(public_key) => {
                let id: [u8; NodeId::UUID_LENGTH] = hash(public_key.to_vec()).lower_bytes();
                NonFungibleGlobalId::new(
                    EDDSA_ED25519_SIGNATURE_VIRTUAL_BADGE,
                    NonFungibleLocalId::bytes(id.to_vec()).unwrap(),
                )
            }
        }
    }
}

//======
// test
//======

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::test_addresses::*;
    use crate::address::Bech32Decoder;

    #[test]
    fn non_fungible_global_id_canonical_conversion() {
        let dec = Bech32Decoder::for_simulator();
        let enc = Bech32Encoder::for_simulator();

        assert_eq!(
            NonFungibleGlobalId::try_from_canonical_string(
                &dec,
                &format!("{NON_FUNGIBLE_RESOURCE_SIM_ADDRESS}:<id>"),
            )
            .unwrap()
            .to_canonical_string(&enc),
            format!("{NON_FUNGIBLE_RESOURCE_SIM_ADDRESS}:<id>")
        );

        assert_eq!(
            NonFungibleGlobalId::try_from_canonical_string(
                &dec,
                &format!("{NON_FUNGIBLE_RESOURCE_SIM_ADDRESS}:#123#"),
            )
            .unwrap()
            .to_canonical_string(&enc),
            format!("{NON_FUNGIBLE_RESOURCE_SIM_ADDRESS}:#123#")
        );

        assert_eq!(
            NonFungibleGlobalId::try_from_canonical_string(
                &dec,
                &format!(
                    "{NON_FUNGIBLE_RESOURCE_SIM_ADDRESS}:{{8fe4abde-affa-4f99-9a0f-300ec6acb64d}}"
                ),
            )
            .unwrap()
            .to_canonical_string(&enc),
            format!("{NON_FUNGIBLE_RESOURCE_SIM_ADDRESS}:{{8fe4abde-affa-4f99-9a0f-300ec6acb64d}}")
        );

        assert_eq!(
            NonFungibleGlobalId::try_from_canonical_string(
                &dec,
                &format!("{NON_FUNGIBLE_RESOURCE_SIM_ADDRESS}:<test>"),
            )
            .unwrap()
            .to_canonical_string(&enc),
            format!("{NON_FUNGIBLE_RESOURCE_SIM_ADDRESS}:<test>"),
        );

        assert_eq!(
            NonFungibleGlobalId::try_from_canonical_string(
                &dec,
                &format!("{NON_FUNGIBLE_RESOURCE_SIM_ADDRESS}:[010a]"),
            )
            .unwrap()
            .to_canonical_string(&enc),
            format!("{NON_FUNGIBLE_RESOURCE_SIM_ADDRESS}:[010a]"),
        );
    }

    #[test]
    fn non_fungible_global_id_canonical_conversion_error() {
        let bech32_decoder = Bech32Decoder::for_simulator();
        assert_eq!(
            NonFungibleGlobalId::try_from_canonical_string(
                &bech32_decoder,
                &NON_FUNGIBLE_RESOURCE_SIM_ADDRESS,
            ),
            Err(ParseNonFungibleGlobalIdError::RequiresTwoParts)
        );

        assert_eq!(
            NonFungibleGlobalId::try_from_canonical_string(
                &bech32_decoder,
                &format!("{NON_FUNGIBLE_RESOURCE_SIM_ADDRESS}:1:2"),
            ),
            Err(ParseNonFungibleGlobalIdError::RequiresTwoParts)
        );

        assert_eq!(
            NonFungibleGlobalId::try_from_canonical_string(
                &bech32_decoder,
                &format!("{NON_FUNGIBLE_RESOURCE_SIM_ADDRESS}:"),
            ),
            Err(ParseNonFungibleGlobalIdError::InvalidNonFungibleLocalId(
                ParseNonFungibleLocalIdError::UnknownType
            ))
        );

        assert!(matches!(
            NonFungibleGlobalId::try_from_canonical_string(&bech32_decoder, ":",),
            Err(ParseNonFungibleGlobalIdError::InvalidResourceAddress)
        ));

        assert!(matches!(
            NonFungibleGlobalId::try_from_canonical_string(
                &bech32_decoder,
                "3nlyju8zsj8h86fz8ma5yl8smwjlg9tckkqvrs520k2p:#1#",
            ),
            Err(ParseNonFungibleGlobalIdError::InvalidResourceAddress)
        ));

        assert!(matches!(
            NonFungibleGlobalId::try_from_canonical_string(
                &bech32_decoder,
                &format!("{NON_FUNGIBLE_RESOURCE_SIM_ADDRESS}:#notnumber#"),
            ),
            Err(ParseNonFungibleGlobalIdError::InvalidNonFungibleLocalId(
                ParseNonFungibleLocalIdError::InvalidInteger
            ))
        ));
    }
}
