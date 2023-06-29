use crate::engine::scrypto_env::ScryptoEnv;
use crate::radix_engine_interface::api::ClientBlueprintApi;
use crate::runtime::Runtime;
use radix_engine_interface::api::node_modules::metadata::MetadataInit;
use radix_engine_interface::api::node_modules::ModuleConfig;
use radix_engine_interface::blueprints::resource::*;
use radix_engine_interface::constants::RESOURCE_PACKAGE;
use radix_engine_interface::data::scrypto::model::*;
use radix_engine_interface::data::scrypto::{scrypto_decode, scrypto_encode};
use radix_engine_interface::math::Decimal;
use radix_engine_interface::types::NonFungibleData;
use radix_engine_interface::types::*;
use radix_engine_interface::*;
use sbor::rust::collections::*;
use sbor::rust::marker::PhantomData;
use scrypto::prelude::ScryptoValue;
use scrypto::resource::ResourceManager;

/// Not divisible.
pub const DIVISIBILITY_NONE: u8 = 0;
/// The maximum divisibility supported.
pub const DIVISIBILITY_MAXIMUM: u8 = 18;

/// Utility for setting up a new resource.
///
/// * You start the building process with one of the methods starting with `new_`.
/// * The allowed methods change depending on which methods have already been called.
///   For example, you can either use `owner_non_fungible_badge` or set access rules individually, but not both.
/// * You can complete the building process using either `create_with_no_initial_supply()` or `mint_initial_supply(..)`.
///
/// ### Example
/// ```no_run
/// use scrypto::prelude::*;
///
/// let bucket = ResourceBuilder::new_fungible(OwnerRole::None)
///     .mint_initial_supply(5);
/// ```
pub struct ResourceBuilder;

impl ResourceBuilder {
    /// Starts a new builder to create a fungible resource.
    pub fn new_fungible(
        owner_role: OwnerRole,
    ) -> InProgressResourceBuilder<FungibleResourceType, NoAuth> {
        InProgressResourceBuilder::new(owner_role)
    }

    /// Starts a new builder to create a non-fungible resource with a `NonFungibleIdType::String`
    pub fn new_string_non_fungible<D: NonFungibleData>(
        owner_role: OwnerRole,
    ) -> InProgressResourceBuilder<NonFungibleResourceType<StringNonFungibleLocalId, D>, NoAuth>
    {
        InProgressResourceBuilder::new(owner_role)
    }

    /// Starts a new builder to create a non-fungible resource with a `NonFungibleIdType::Integer`
    pub fn new_integer_non_fungible<D: NonFungibleData>(
        owner_role: OwnerRole,
    ) -> InProgressResourceBuilder<NonFungibleResourceType<IntegerNonFungibleLocalId, D>, NoAuth>
    {
        InProgressResourceBuilder::new(owner_role)
    }

    /// Starts a new builder to create a non-fungible resource with a `NonFungibleIdType::Bytes`
    pub fn new_bytes_non_fungible<D: NonFungibleData>(
        owner_role: OwnerRole,
    ) -> InProgressResourceBuilder<NonFungibleResourceType<BytesNonFungibleLocalId, D>, NoAuth>
    {
        InProgressResourceBuilder::new(owner_role)
    }

    /// Starts a new builder to create a non-fungible resource with a `NonFungibleIdType::RUID`
    pub fn new_ruid_non_fungible<D: NonFungibleData>(
        owner_role: OwnerRole,
    ) -> InProgressResourceBuilder<NonFungibleResourceType<RUIDNonFungibleLocalId, D>, NoAuth> {
        InProgressResourceBuilder::new(owner_role)
    }
}

/// Utility for setting up a new resource, which has building in progress.
///
/// * You start the building process with one of the methods starting with `ResourceBuilder::new_`.
/// * The allowed methods change depending on which methods have already been called.
///   For example, you can either use `owner_non_fungible_badge` or set access rules individually, but not both.
/// * You can complete the building process using either `create_with_no_initial_supply()` or `mint_initial_supply(..)`.
///
/// ### Example
/// ```no_run
/// use scrypto::prelude::*;
///
/// let bucket = ResourceBuilder::new_fungible(OwnerRole::None)
///     .mint_initial_supply(5);
/// ```
#[must_use]
pub struct InProgressResourceBuilder<T: AnyResourceType, A: ResourceConfig> {
    owner_role: OwnerRole,
    resource_type: T,
    metadata_config: Option<ModuleConfig<MetadataInit>>,
    address_reservation: Option<GlobalAddressReservation>,
    config: A,
}

impl<T: AnyResourceType> InProgressResourceBuilder<T, NoAuth> {
    fn new(owner_role: OwnerRole) -> Self {
        Self {
            owner_role,
            resource_type: T::default(),
            metadata_config: None,
            address_reservation: None,
            config: NoAuth,
        }
    }
}

pub trait ResourceConfig {
    fn into_supported_actions_and_roles(self) -> (BTreeSet<ResourceAction>, RolesInit);
}

pub struct NoAuth;
impl ResourceConfig for NoAuth {
    fn into_supported_actions_and_roles(self) -> (BTreeSet<ResourceAction>, RolesInit) {
        (
            BTreeSet::new(),
            roles_init! {}
        )
    }
}

pub struct ResourceActionRolesInit(BTreeSet<ResourceAction>, RolesInit);

impl ResourceConfig for ResourceActionRolesInit {
    fn into_supported_actions_and_roles(self) -> (BTreeSet<ResourceAction>, RolesInit) {
        (self.0, self.1)
    }
}

// Various types for ResourceType
pub trait AnyResourceType: Default {}

pub struct FungibleResourceType {
    divisibility: u8,
}
impl AnyResourceType for FungibleResourceType {}
impl Default for FungibleResourceType {
    fn default() -> Self {
        Self {
            divisibility: DIVISIBILITY_MAXIMUM,
        }
    }
}

pub struct NonFungibleResourceType<T: IsNonFungibleLocalId, D: NonFungibleData>(
    PhantomData<T>,
    PhantomData<D>,
);
impl<T: IsNonFungibleLocalId, D: NonFungibleData> AnyResourceType
    for NonFungibleResourceType<T, D>
{
}
impl<T: IsNonFungibleLocalId, D: NonFungibleData> Default for NonFungibleResourceType<T, D> {
    fn default() -> Self {
        Self(PhantomData, PhantomData)
    }
}

// Builder types
pub trait IsFungibleBuilder {}
impl<A: ResourceConfig> IsFungibleBuilder for InProgressResourceBuilder<FungibleResourceType, A> {}

pub trait IsNonFungibleBuilder {}
impl<A: ResourceConfig, Y: IsNonFungibleLocalId, D: NonFungibleData> IsNonFungibleBuilder
    for InProgressResourceBuilder<NonFungibleResourceType<Y, D>, A>
{
}

////////////////////////////////////////////////////////////
/// PUBLIC TRAITS AND METHODS
/// All public methods first - these all need good rust docs
////////////////////////////////////////////////////////////

pub trait UpdateMetadataBuilder: private::CanSetMetadata {
    fn metadata(self, metadata: ModuleConfig<MetadataInit>) -> Self::OutputBuilder {
        self.set_metadata(metadata)
    }
}
impl<B: private::CanSetMetadata> UpdateMetadataBuilder for B {}

pub trait SetAddressReservationBuilder: private::CanSetAddressReservation {
    /// Sets the address reservation
    fn with_address(self, reservation: GlobalAddressReservation) -> Self::OutputBuilder {
        self.set_address(reservation)
    }
}
impl<B: private::CanSetAddressReservation> SetAddressReservationBuilder for B {}

pub trait UpdateAuthBuilder: private::CanAddAuth {
    /// Sets the resource to be mintable.
    ///
    /// * The first parameter is the access rule which allows minting of the resource.
    /// * The second parameter is the mutability / access rule which controls if and how the access rule can be updated.
    ///
    /// ### Examples
    ///
    /// ```no_run
    /// use radix_engine_interface::mintable;
    /// use scrypto::prelude::*;
    ///
    /// # let resource_address = RADIX_TOKEN;
    /// // Sets the resource to be mintable with a proof of a specific resource, and this is locked forever.
    /// ResourceBuilder::new_fungible(OwnerRole::None)
    ///    .mintable(mintable! {
    ///         minter => rule!(require(resource_address)), locked;
    ///         minter_updater => rule!(deny_all), locked;
    ///     });
    ///
    /// # let resource_address = RADIX_TOKEN;
    /// // Sets the resource to not be mintable, but this is can be changed in future by the second rule
    /// ResourceBuilder::new_fungible(OwnerRole::None)
    ///    .mintable(mintable! {
    ///         minter => rule!(deny_all), updatable;
    ///         minter_updater => rule!(require(resource_address)), locked;
    ///    });
    /// ```
    fn mintable(self, role_init: RolesInit) -> Self::OutputBuilder {
        self.add_auth(Mint, role_init)
    }

    /// Sets the resource to be burnable.
    ///
    /// * The first parameter is the access rule which allows minting of the resource.
    /// * The second parameter is the mutability / access rule which controls if and how the access rule can be updated.
    ///
    /// ### Examples
    ///
    /// ```no_run
    /// use radix_engine_interface::burnable;
    /// use scrypto::prelude::*;
    ///
    /// # let resource_address = RADIX_TOKEN;
    /// // Sets the resource to be burnable with a proof of a specific resource, and this is locked forever.
    /// ResourceBuilder::new_fungible(OwnerRole::None)
    ///    .burnable(burnable! {
    ///        burner => rule!(require(resource_address)), locked;
    ///        burner_updater => rule!(deny_all), locked;
    ///    });
    ///
    /// # let resource_address = RADIX_TOKEN;
    /// // Sets the resource to be freely burnable, but this is can be changed in future by the second rule.
    /// ResourceBuilder::new_fungible(OwnerRole::None)
    ///    .burnable(burnable! {
    ///        burner => rule!(allow_all), updatable;
    ///        burner_updater => rule!(require(resource_address)), updatable;
    ///    });
    /// ```
    fn burnable(self, role_init: RolesInit) -> Self::OutputBuilder {
        self.add_auth(Burn, role_init)
    }

    /// Sets the resource to be recallable from vaults.
    ///
    /// * The first parameter is the access rule which allows recalling of the resource.
    /// * The second parameter is the mutability / access rule which controls if and how the access rule can be updated.
    ///
    /// ### Examples
    ///
    /// ```no_run
    /// use scrypto::prelude::*;
    ///
    /// # let resource_address = RADIX_TOKEN;
    /// // Sets the resource to be recallable with a proof of a specific resource, and this is locked forever.
    /// ResourceBuilder::new_fungible(OwnerRole::None)
    ///    .recallable(recallable! {
    ///        recaller => rule!(require(resource_address)), locked;
    ///        recaller_updater => rule!(deny_all), locked;
    ///    });
    ///
    /// # let resource_address = RADIX_TOKEN;
    /// // Sets the resource to not be recallable, but this is can be changed in future by the second rule
    /// ResourceBuilder::new_fungible(OwnerRole::None)
    ///    .recallable(recallable! {
    ///        recaller => rule!(deny_all), updatable;
    ///        recaller_updater => rule!(require(resource_address)), updatable;
    ///    });
    /// ```
    fn recallable(self, role_init: RolesInit) -> Self::OutputBuilder {
        self.add_auth(Recall, role_init)
    }

    /// Sets the resource to have freezeable.
    ///
    /// * The first parameter is the access rule which allows freezing of the vault.
    /// * The second parameter is the mutability / access rule which controls if and how the access rule can be updated.
    ///
    /// ### Examples
    ///
    /// ```no_run
    /// use radix_engine_interface::freezable;
    /// use scrypto::prelude::*;
    ///
    /// # let resource_address = RADIX_TOKEN;
    /// // Sets the resource to be freezeable with a proof of a specific resource, and this is locked forever.
    /// ResourceBuilder::new_fungible(OwnerRole::None)
    ///    .freezeable(freezable! {
    ///        freezer: rule!(require(resource_address)), locked;
    ///        freezer_updater: rule!(deny_all), locked;
    ///    });
    ///
    /// # let resource_address = RADIX_TOKEN;
    /// // Sets the resource to not be freezeable, but this is can be changed in future by the second rule
    /// ResourceBuilder::new_fungible(OwnerRole::None)
    ///    .freezeable(freezable! {
    ///        freezer: rule!(deny_all), updatable;
    ///        freezer_updater: rule!(require(resource_address)), updatable;
    ///    });
    /// ```
    fn freezeable(self, role_init: RolesInit) -> Self::OutputBuilder {
        self.add_auth(Freeze, role_init)
    }

    /// Sets the resource to not be freely withdrawable from a vault.
    ///
    /// * The first parameter is the access rule which allows withdrawing from a vault.
    /// * The second parameter is the mutability / access rule which controls if and how the access rule can be updated.
    ///
    /// ### Examples
    ///
    /// ```no_run
    /// use radix_engine_interface::restrict_withdraw;
    /// use scrypto::prelude::*;
    ///
    /// # let resource_address = RADIX_TOKEN;
    /// // Sets the resource to be withdrawable with a proof of a specific resource, and this is locked forever.
    /// ResourceBuilder::new_fungible(OwnerRole::None)
    ///    .restrict_withdraw(restrict_withdraw! {
    ///        withdrawer => rule!(require(resource_address)), locked;
    ///        withdrawer_updater => rule!(deny_all), locked;
    ///    });
    ///
    /// # let resource_address = RADIX_TOKEN;
    /// // Sets the resource to not be withdrawable, but this is can be changed in future by the second rule
    /// ResourceBuilder::new_fungible(OwnerRole::None)
    ///    .restrict_withdraw(restrict_withdraw! {
    ///        withdrawer => rule!(deny_all), updatable;
    ///        withdrawer_updater => rule!(require(resource_address)), updatable;
    ///    });
    /// ```
    fn restrict_withdraw(self, role_init: RolesInit) -> Self::OutputBuilder {
        self.add_auth(Withdraw, role_init)
    }

    /// Sets the resource to not be freely depositable into a vault.
    ///
    /// * The first parameter is the access rule which allows depositing into a vault.
    /// * The second parameter is the mutability / access rule which controls if and how the access rule can be updated.
    ///
    /// ### Examples
    ///
    /// ```no_run
    /// use scrypto::prelude::*;
    ///
    /// # let resource_address = RADIX_TOKEN;
    /// // Sets the resource to be depositable with a proof of a specific resource, and this is locked forever.
    /// ResourceBuilder::new_fungible(OwnerRole::None)
    ///    .restrict_deposit(restrict_deposit! {
    ///        depositor => rule!(require(resource_address)), locked;
    ///        depositor_updater => rule!(deny_all), locked;
    ///    });
    ///
    /// # let resource_address = RADIX_TOKEN;
    /// // Sets the resource to not be depositable, but this is can be changed in future by the second rule
    /// ResourceBuilder::new_fungible(OwnerRole::None)
    ///    .restrict_deposit(restrict_deposit! {
    ///        depositor => rule!(deny_all), locked;
    ///        depositor_updater => rule!(require(resource_address)), locked;
    ///    });
    /// ```
    fn restrict_deposit(self, role_init: RolesInit) -> Self::OutputBuilder {
        self.add_auth(Deposit, role_init)
    }
}
impl<B: private::CanAddAuth> UpdateAuthBuilder for B {}

pub trait UpdateNonFungibleAuthBuilder: IsNonFungibleBuilder + private::CanAddAuth {
    /// Sets how each non-fungible's mutable data can be updated.
    ///
    /// * The first parameter is the access rule which allows updating the mutable data of each non-fungible.
    /// * The second parameter is the mutability / access rule which controls if and how the access rule can be updated.
    ///
    /// ### Examples
    ///
    /// ```no_run
    /// use radix_engine_interface::updatable_non_fungible_data;
    /// use scrypto::prelude::*;
    ///
    /// # let resource_address = RADIX_TOKEN;
    ///
    /// #[derive(ScryptoSbor, NonFungibleData)]
    /// struct NFData {
    ///     pub name: String,
    ///     #[mutable]
    ///     pub flag: bool,
    /// }
    /// // Permits the updating of non-fungible mutable data with a proof of a specific resource, and this is locked forever.
    /// ResourceBuilder::new_ruid_non_fungible::<NFData>(OwnerRole::None)
    ///    .updatable_non_fungible_data(updatable_non_fungible_data! {
    ///        non_fungible_data_updater => rule!(require(resource_address)), locked;
    ///        non_fungible_data_updater_updater => rule!(deny_all), locked;
    ///    });
    ///
    /// # let resource_address = RADIX_TOKEN;
    /// // Does not currently permit the updating of non-fungible mutable data, but this is can be changed in future by the second rule.
    /// ResourceBuilder::new_ruid_non_fungible::<NFData>(OwnerRole::None)
    ///    .updatable_non_fungible_data(updatable_non_fungible_data! {
    ///        non_fungible_data_updater => rule!(deny_all), updatable;
    ///        non_fungible_data_updater_updater => rule!(require(resource_address)), updatable;
    ///    });
    /// ```
    fn updatable_non_fungible_data<R: Into<AccessRule>>(
        self,
        role_init: RolesInit,
    ) -> Self::OutputBuilder {
        self.add_auth(
            UpdateNonFungibleData,
            role_init,
        )
    }
}
impl<B: IsNonFungibleBuilder + private::CanAddAuth> UpdateNonFungibleAuthBuilder for B {}

pub trait SetOwnerBuilder: private::CanAddOwner {
    /// Sets the owner badge to be the given non-fungible.
    ///
    /// The owner badge is given starting permissions to update the metadata/data associated with the resource,
    /// and to change any of the access rules after creation.
    fn owner_non_fungible_badge(self, owner_badge: NonFungibleGlobalId) -> Self::OutputBuilder {
        self.set_owner(owner_badge)
    }
}
impl<B: private::CanAddOwner> SetOwnerBuilder for B {}

pub trait CreateWithNoSupplyBuilder: private::CanCreateWithNoSupply {
    /// Creates the resource with no initial supply.
    ///
    /// The resource's address is returned.
    fn create_with_no_initial_supply(self) -> ResourceManager {
        match self.into_create_with_no_supply_invocation() {
            private::CreateWithNoSupply::Fungible {
                owner_role,
                divisibility,
                supported_actions,
                roles,
                metadata,
                address_reservation,
            } => {
                let metadata = metadata.unwrap_or_else(|| Default::default());

                ScryptoEnv
                    .call_function(
                        RESOURCE_PACKAGE,
                        FUNGIBLE_RESOURCE_MANAGER_BLUEPRINT,
                        FUNGIBLE_RESOURCE_MANAGER_CREATE_IDENT,
                        scrypto_encode(&FungibleResourceManagerCreateInput {
                            owner_role,
                            divisibility,
                            track_total_supply: true,
                            metadata,
                            supported_actions,
                            roles,
                            address_reservation,
                        })
                        .unwrap(),
                    )
                    .map(|bytes| scrypto_decode(&bytes).unwrap())
                    .unwrap()
            }
            private::CreateWithNoSupply::NonFungible {
                owner_role,
                id_type,
                non_fungible_schema,
                supported_actions,
                roles,
                metadata,
                address_reservation,
            } => {
                let metadata = metadata.unwrap_or_else(|| Default::default());

                ScryptoEnv
                    .call_function(
                        RESOURCE_PACKAGE,
                        NON_FUNGIBLE_RESOURCE_MANAGER_BLUEPRINT,
                        NON_FUNGIBLE_RESOURCE_MANAGER_CREATE_IDENT,
                        scrypto_encode(&NonFungibleResourceManagerCreateInput {
                            owner_role,
                            id_type,
                            track_total_supply: true,
                            non_fungible_schema,
                            supported_actions,
                            roles,
                            metadata,
                            address_reservation,
                        })
                        .unwrap(),
                    )
                    .map(|bytes| scrypto_decode(&bytes).unwrap())
                    .unwrap()
            }
        }
    }
}
impl<B: private::CanCreateWithNoSupply> CreateWithNoSupplyBuilder for B {}

impl<A: ResourceConfig> InProgressResourceBuilder<FungibleResourceType, A> {
    /// Set the resource's divisibility: the number of digits of precision after the decimal point in its balances.
    ///
    /// * `0` means the resource is not divisible (balances are always whole numbers)
    /// * `18` is the maximum divisibility, and the default.
    ///
    /// ### Examples
    ///
    /// ```no_run
    /// use scrypto::prelude::*;
    ///
    /// // Only permits whole-number balances.
    /// ResourceBuilder::new_fungible(OwnerRole::None)
    ///    .divisibility(0);
    ///
    /// // Only permits balances to 3 decimal places.
    /// ResourceBuilder::new_fungible(OwnerRole::None)
    ///    .divisibility(3);
    /// ```
    pub fn divisibility(mut self, divisibility: u8) -> Self {
        assert!(divisibility <= 18);
        self.resource_type = FungibleResourceType { divisibility };
        self
    }
}

impl<A: ResourceConfig> InProgressResourceBuilder<FungibleResourceType, A> {
    /// Creates resource with the given initial supply.
    ///
    /// # Example
    /// ```no_run
    /// use scrypto::prelude::*;
    ///
    /// let bucket = ResourceBuilder::new_fungible(OwnerRole::None)
    ///     .mint_initial_supply(5);
    /// ```
    pub fn mint_initial_supply<T: Into<Decimal>>(mut self, amount: T) -> Bucket {

        let (supported_actions, roles) = self.config.into_supported_actions_and_roles();

        let metadata = self
            .metadata_config
            .take()
            .unwrap_or_else(|| Default::default());

        ScryptoEnv
            .call_function(
                RESOURCE_PACKAGE,
                FUNGIBLE_RESOURCE_MANAGER_BLUEPRINT,
                FUNGIBLE_RESOURCE_MANAGER_CREATE_WITH_INITIAL_SUPPLY_IDENT,
                scrypto_encode(&FungibleResourceManagerCreateWithInitialSupplyInput {
                    owner_role: self.owner_role,
                    track_total_supply: true,
                    divisibility: self.resource_type.divisibility,
                    supported_actions,
                    roles,
                    metadata,
                    initial_supply: amount.into(),
                    address_reservation: self.address_reservation,
                })
                .unwrap(),
            )
            .map(|bytes| {
                scrypto_decode::<(ResourceAddress, Bucket)>(&bytes)
                    .unwrap()
                    .1
            })
            .unwrap()
    }
}

impl<A: ResourceConfig, D: NonFungibleData>
    InProgressResourceBuilder<NonFungibleResourceType<StringNonFungibleLocalId, D>, A>
{
    /// Creates the non-fungible resource, and mints an individual non-fungible for each key/data pair provided.
    ///
    /// ### Example
    /// ```no_run
    /// use scrypto::prelude::*;
    ///
    /// #[derive(ScryptoSbor, NonFungibleData)]
    /// struct NFData {
    ///     pub name: String,
    ///     #[mutable]
    ///     pub flag: bool,
    /// }
    ///
    /// let bucket = ResourceBuilder::new_string_non_fungible::<NFData>(OwnerRole::None)
    ///     .mint_initial_supply([
    ///         ("One".try_into().unwrap(), NFData { name: "NF One".to_owned(), flag: true }),
    ///         ("Two".try_into().unwrap(), NFData { name: "NF Two".to_owned(), flag: true }),
    ///     ]);
    /// ```
    pub fn mint_initial_supply<T>(mut self, entries: T) -> Bucket
    where
        T: IntoIterator<Item = (StringNonFungibleLocalId, D)>,
    {
        let mut non_fungible_schema = NonFungibleDataSchema::new_schema::<D>();
        non_fungible_schema.replace_self_package_address(Runtime::package_address());

        let (supported_actions, roles) = self.config.into_supported_actions_and_roles();

        let metadata = self
            .metadata_config
            .take()
            .unwrap_or_else(|| Default::default());

        ScryptoEnv
            .call_function(
                RESOURCE_PACKAGE,
                NON_FUNGIBLE_RESOURCE_MANAGER_BLUEPRINT,
                NON_FUNGIBLE_RESOURCE_MANAGER_CREATE_WITH_INITIAL_SUPPLY_IDENT,
                scrypto_encode(&NonFungibleResourceManagerCreateWithInitialSupplyInput {
                    owner_role: self.owner_role,
                    track_total_supply: true,
                    id_type: StringNonFungibleLocalId::id_type(),
                    non_fungible_schema,
                    supported_actions,
                    roles,
                    metadata,
                    entries: map_entries(entries),
                    address_reservation: self.address_reservation,
                })
                .unwrap(),
            )
            .map(|bytes| {
                scrypto_decode::<(ResourceAddress, Bucket)>(&bytes)
                    .unwrap()
                    .1
            })
            .unwrap()
    }
}

impl<A: ResourceConfig, D: NonFungibleData>
    InProgressResourceBuilder<NonFungibleResourceType<IntegerNonFungibleLocalId, D>, A>
{
    /// Creates the non-fungible resource, and mints an individual non-fungible for each key/data pair provided.
    ///
    /// ### Example
    /// ```no_run
    /// use scrypto::prelude::*;
    ///
    /// #[derive(ScryptoSbor, NonFungibleData)]
    /// struct NFData {
    ///     pub name: String,
    ///     #[mutable]
    ///     pub flag: bool,
    /// }
    ///
    /// let bucket = ResourceBuilder::new_integer_non_fungible(OwnerRole::None)
    ///     .mint_initial_supply([
    ///         (1u64.into(), NFData { name: "NF One".to_owned(), flag: true }),
    ///         (2u64.into(), NFData { name: "NF Two".to_owned(), flag: true }),
    ///     ]);
    /// ```
    pub fn mint_initial_supply<T>(mut self, entries: T) -> Bucket
    where
        T: IntoIterator<Item = (IntegerNonFungibleLocalId, D)>,
    {
        let mut non_fungible_schema = NonFungibleDataSchema::new_schema::<D>();
        non_fungible_schema.replace_self_package_address(Runtime::package_address());

        let (supported_actions, roles) = self.config.into_supported_actions_and_roles();

        let metadata = self
            .metadata_config
            .take()
            .unwrap_or_else(|| Default::default());

        ScryptoEnv
            .call_function(
                RESOURCE_PACKAGE,
                NON_FUNGIBLE_RESOURCE_MANAGER_BLUEPRINT,
                NON_FUNGIBLE_RESOURCE_MANAGER_CREATE_WITH_INITIAL_SUPPLY_IDENT,
                scrypto_encode(&NonFungibleResourceManagerCreateWithInitialSupplyInput {
                    owner_role: self.owner_role,
                    track_total_supply: true,
                    id_type: IntegerNonFungibleLocalId::id_type(),
                    non_fungible_schema,
                    supported_actions,
                    roles,
                    metadata,
                    entries: map_entries(entries),
                    address_reservation: self.address_reservation,
                })
                .unwrap(),
            )
            .map(|bytes| {
                scrypto_decode::<(ResourceAddress, Bucket)>(&bytes)
                    .unwrap()
                    .1
            })
            .unwrap()
    }
}

impl<A: ResourceConfig, D: NonFungibleData>
    InProgressResourceBuilder<NonFungibleResourceType<BytesNonFungibleLocalId, D>, A>
{
    /// Creates the non-fungible resource, and mints an individual non-fungible for each key/data pair provided.
    ///
    /// ### Example
    /// ```no_run
    /// use scrypto::prelude::*;
    ///
    /// #[derive(ScryptoSbor, NonFungibleData)]
    /// struct NFData {
    ///     pub name: String,
    ///     #[mutable]
    ///     pub flag: bool,
    /// }
    ///
    /// let bucket = ResourceBuilder::new_bytes_non_fungible::<NFData>(OwnerRole::None)
    ///     .mint_initial_supply([
    ///         (vec![1u8].try_into().unwrap(), NFData { name: "NF One".to_owned(), flag: true }),
    ///         (vec![2u8].try_into().unwrap(), NFData { name: "NF Two".to_owned(), flag: true }),
    ///     ]);
    /// ```
    pub fn mint_initial_supply<T>(mut self, entries: T) -> Bucket
    where
        T: IntoIterator<Item = (BytesNonFungibleLocalId, D)>,
    {
        let mut non_fungible_schema = NonFungibleDataSchema::new_schema::<D>();
        non_fungible_schema.replace_self_package_address(Runtime::package_address());

        let (supported_actions, roles) = self.config.into_supported_actions_and_roles();

        let metadata = self
            .metadata_config
            .take()
            .unwrap_or_else(|| Default::default());

        ScryptoEnv
            .call_function(
                RESOURCE_PACKAGE,
                NON_FUNGIBLE_RESOURCE_MANAGER_BLUEPRINT,
                NON_FUNGIBLE_RESOURCE_MANAGER_CREATE_WITH_INITIAL_SUPPLY_IDENT,
                scrypto_encode(&NonFungibleResourceManagerCreateWithInitialSupplyInput {
                    owner_role: self.owner_role,
                    id_type: BytesNonFungibleLocalId::id_type(),
                    track_total_supply: true,
                    non_fungible_schema,
                    supported_actions,
                    roles,
                    metadata,
                    entries: map_entries(entries),
                    address_reservation: self.address_reservation,
                })
                .unwrap(),
            )
            .map(|bytes| {
                scrypto_decode::<(ResourceAddress, Bucket)>(&bytes)
                    .unwrap()
                    .1
            })
            .unwrap()
    }
}

impl<A: ResourceConfig, D: NonFungibleData>
    InProgressResourceBuilder<NonFungibleResourceType<RUIDNonFungibleLocalId, D>, A>
{
    /// Creates the RUID non-fungible resource, and mints an individual non-fungible for each piece of data provided.
    ///
    /// The system automatically generates a new RUID `NonFungibleLocalId` for each non-fungible,
    /// and assigns the given data to each.
    ///
    /// ### Example
    /// ```no_run
    /// use scrypto::prelude::*;
    ///
    /// #[derive(ScryptoSbor, NonFungibleData)]
    /// struct NFData {
    ///     pub name: String,
    ///     #[mutable]
    ///     pub flag: bool,
    /// }
    ///
    /// let bucket = ResourceBuilder::new_ruid_non_fungible::<NFData>(OwnerRole::None)
    ///     .mint_initial_supply([
    ///         (NFData { name: "NF One".to_owned(), flag: true }),
    ///         (NFData { name: "NF Two".to_owned(), flag: true }),
    ///     ]);
    /// ```
    pub fn mint_initial_supply<T>(mut self, entries: T) -> Bucket
    where
        T: IntoIterator<Item = D>,
    {
        let mut non_fungible_schema = NonFungibleDataSchema::new_schema::<D>();
        non_fungible_schema.replace_self_package_address(Runtime::package_address());

        let metadata = self
            .metadata_config
            .take()
            .unwrap_or_else(|| Default::default());

        let (supported_actions, roles) = self.config.into_supported_actions_and_roles();

        ScryptoEnv
            .call_function(
                RESOURCE_PACKAGE,
                NON_FUNGIBLE_RESOURCE_MANAGER_BLUEPRINT,
                NON_FUNGIBLE_RESOURCE_MANAGER_CREATE_RUID_WITH_INITIAL_SUPPLY_IDENT,
                scrypto_encode(
                    &NonFungibleResourceManagerCreateRuidWithInitialSupplyInput {
                        owner_role: self.owner_role,
                        non_fungible_schema,
                        track_total_supply: true,
                        supported_actions,
                        roles,
                        metadata,
                        entries: entries
                            .into_iter()
                            .map(|data| {
                                let value: ScryptoValue =
                                    scrypto_decode(&scrypto_encode(&data).unwrap()).unwrap();
                                (value,)
                            })
                            .collect(),
                        address_reservation: self.address_reservation,
                    },
                )
                .unwrap(),
            )
            .map(|bytes| {
                scrypto_decode::<(ResourceAddress, Bucket)>(&bytes)
                    .unwrap()
                    .1
            })
            .unwrap()
    }
}

///////////////////////////////////
/// PRIVATE TRAIT IMPLEMENTATIONS
/// These don't need good rust docs
///////////////////////////////////

fn map_entries<T: IntoIterator<Item = (Y, V)>, V: NonFungibleData, Y: IsNonFungibleLocalId>(
    entries: T,
) -> BTreeMap<NonFungibleLocalId, (ScryptoValue,)> {
    entries
        .into_iter()
        .map(|(id, data)| {
            let value: ScryptoValue = scrypto_decode(&scrypto_encode(&data).unwrap()).unwrap();
            (id.into(), (value,))
        })
        .collect()
}

impl<T: AnyResourceType, A: ResourceConfig> private::CanSetMetadata
    for InProgressResourceBuilder<T, A>
{
    type OutputBuilder = Self;

    fn set_metadata(mut self, metadata: ModuleConfig<MetadataInit>) -> Self::OutputBuilder {
        self.metadata_config = Some(metadata);
        self
    }
}

impl<T: AnyResourceType, A: ResourceConfig> private::CanSetAddressReservation
    for InProgressResourceBuilder<T, A>
{
    type OutputBuilder = Self;

    fn set_address(mut self, address_reservation: GlobalAddressReservation) -> Self::OutputBuilder {
        self.address_reservation = Some(address_reservation);
        self
    }
}

impl<T: AnyResourceType> private::CanAddAuth for InProgressResourceBuilder<T, NoAuth> {
    type OutputBuilder = InProgressResourceBuilder<T, ResourceActionRolesInit>;

    fn add_auth(
        self,
        action: ResourceAction,
        role_init: RolesInit,
    ) -> Self::OutputBuilder {
        Self::OutputBuilder {
            owner_role: self.owner_role,
            resource_type: self.resource_type,
            config: ResourceActionRolesInit(btreeset!(action), role_init),
            metadata_config: self.metadata_config,
            address_reservation: self.address_reservation,
        }
    }
}

impl<T: AnyResourceType> private::CanAddAuth
    for InProgressResourceBuilder<T, ResourceActionRolesInit>
{
    type OutputBuilder = Self;

    fn add_auth(
        mut self,
        action: ResourceAction,
        role_init: RolesInit,
    ) -> Self::OutputBuilder {
        self.config.0.insert(action);
        self.config.1.data.extend(role_init.data);
        self
    }
}

impl<A: ResourceConfig> private::CanCreateWithNoSupply
    for InProgressResourceBuilder<FungibleResourceType, A>
{
    fn into_create_with_no_supply_invocation(self) -> private::CreateWithNoSupply {
        let (supported_actions, roles) = self.config.into_supported_actions_and_roles();

        private::CreateWithNoSupply::Fungible {
            owner_role: self.owner_role,
            divisibility: self.resource_type.divisibility,
            supported_actions,
            roles,
            metadata: self.metadata_config,
            address_reservation: self.address_reservation,
        }
    }
}

impl<A: ResourceConfig, Y: IsNonFungibleLocalId, D: NonFungibleData> private::CanCreateWithNoSupply
    for InProgressResourceBuilder<NonFungibleResourceType<Y, D>, A>
{
    fn into_create_with_no_supply_invocation(self) -> private::CreateWithNoSupply {
        let mut non_fungible_schema = NonFungibleDataSchema::new_schema::<D>();
        non_fungible_schema.replace_self_package_address(Runtime::package_address());

        let (supported_actions, roles) = self.config.into_supported_actions_and_roles();

        private::CreateWithNoSupply::NonFungible {
            owner_role: self.owner_role,
            id_type: Y::id_type(),
            non_fungible_schema,
            supported_actions,
            roles,
            metadata: self.metadata_config,
            address_reservation: self.address_reservation,
        }
    }
}

/// This file was experiencing combinatorial explosion - as part of the clean-up, we've used private traits to keep things simple.
///
/// Each public method has essentially one implementation, and one Rust doc (where there weren't clashes due to Rust trait issues -
/// eg with the `mint_initial_supply` methods).
///
/// Internally, the various builders implement these private traits, and then automatically implement the "nice" public traits.
/// The methods defined in the private traits are less nice, and so are hidden in order to not pollute the user facing API.
///
/// As users will nearly always use `scrypto::prelude::*`, as long as we make sure that the public traits are exported, this will
/// be seamless for the user.
///
/// See https://stackoverflow.com/a/53207767 for more information on this.
mod private {
    use super::*;
    use radix_engine_interface::blueprints::resource::{NonFungibleGlobalId, ResourceAction};

    pub trait CanSetMetadata: Sized {
        type OutputBuilder;

        fn set_metadata(self, metadata: ModuleConfig<MetadataInit>) -> Self::OutputBuilder;
    }

    pub trait CanSetAddressReservation: Sized {
        type OutputBuilder;

        fn set_address(self, address_reservation: GlobalAddressReservation) -> Self::OutputBuilder;
    }

    pub trait CanAddAuth: Sized {
        type OutputBuilder;

        fn add_auth(
            self,
            method: ResourceAction,
            role_init: RolesInit,
        ) -> Self::OutputBuilder;
    }

    pub trait CanAddOwner: Sized {
        type OutputBuilder;

        fn set_owner(self, owner_badge: NonFungibleGlobalId) -> Self::OutputBuilder;
    }

    pub trait CanCreateWithNoSupply: Sized {
        fn into_create_with_no_supply_invocation(self) -> CreateWithNoSupply;
    }

    pub enum CreateWithNoSupply {
        Fungible {
            owner_role: OwnerRole,
            divisibility: u8,
            supported_actions: BTreeSet<ResourceAction>,
            roles: RolesInit,
            metadata: Option<ModuleConfig<MetadataInit>>,
            address_reservation: Option<GlobalAddressReservation>,
        },
        NonFungible {
            owner_role: OwnerRole,
            id_type: NonFungibleIdType,
            non_fungible_schema: NonFungibleDataSchema,
            supported_actions: BTreeSet<ResourceAction>,
            roles: RolesInit,
            metadata: Option<ModuleConfig<MetadataInit>>,
            address_reservation: Option<GlobalAddressReservation>,
        },
    }
}
