use crate::engine::{
    deref_and_update, ApplicationError, CallFrameUpdate, ExecutableInvocation, Executor, LockFlags,
    RENode, ResolvedActor, ResolverApi, RuntimeError, SystemApi,
};
use crate::model::{
    AccessRulesChainSubstate, GlobalAddressSubstate, HardAuthRule, HardProofRule,
    HardResourceOrNonFungible, MethodAuthorization, ValidatorSubstate,
};
use crate::types::*;
use crate::wasm::WasmEngine;
use native_sdk::resource::{ResourceManager, Vault};
use radix_engine_interface::api::api::{EngineApi, InvokableModel};
use radix_engine_interface::api::types::{
    EpochManagerFn, EpochManagerOffset, GlobalAddress, NativeFn, RENodeId, SubstateOffset,
};
use radix_engine_interface::model::*;
use radix_engine_interface::modules::auth::AuthAddresses;
use radix_engine_interface::rule;

#[derive(Debug, Clone, PartialEq, Eq, ScryptoCategorize, ScryptoEncode, ScryptoDecode)]
pub struct EpochManagerSubstate {
    pub address: ComponentAddress, // TODO: Does it make sense for this to be stored here?
    pub epoch: u64,
    pub round: u64,

    // TODO: Move configuration to an immutable substate
    pub rounds_per_epoch: u64,
    pub num_unstake_epochs: u64,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Ord, PartialOrd, ScryptoCategorize, ScryptoEncode, ScryptoDecode,
)]
pub struct Validator {
    pub key: EcdsaSecp256k1PublicKey,
    pub stake: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, ScryptoCategorize, ScryptoEncode, ScryptoDecode)]
pub struct ValidatorSetSubstate {
    pub validator_set: BTreeMap<ComponentAddress, Validator>,
    pub epoch: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Categorize, Encode, Decode)]
pub enum EpochManagerError {
    InvalidRoundUpdate { from: u64, to: u64 },
}

pub struct EpochManager;

impl ExecutableInvocation for EpochManagerCreateInvocation {
    type Exec = Self;

    fn resolve<D: ResolverApi>(
        self,
        _deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let actor = ResolvedActor::function(NativeFn::EpochManager(EpochManagerFn::Create));

        let mut call_frame_update =
            CallFrameUpdate::copy_ref(RENodeId::Global(GlobalAddress::Resource(RADIX_TOKEN)));
        for bucket in self.validator_set.values() {
            call_frame_update
                .nodes_to_move
                .push(RENodeId::Bucket(bucket.0));
        }

        Ok((actor, call_frame_update, self))
    }
}

impl Executor for EpochManagerCreateInvocation {
    type Output = ComponentAddress;

    fn execute<Y, W: WasmEngine>(
        self,
        api: &mut Y,
    ) -> Result<(Self::Output, CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi + EngineApi<RuntimeError> + InvokableModel<RuntimeError>,
    {
        let underlying_node_id = api.allocate_node_id(RENodeType::EpochManager)?;
        let global_node_id = RENodeId::Global(GlobalAddress::Component(
            ComponentAddress::EpochManager(self.component_address),
        ));

        let epoch_manager = EpochManagerSubstate {
            address: global_node_id.into(),
            epoch: self.initial_epoch,
            round: 0,
            rounds_per_epoch: self.rounds_per_epoch,
            num_unstake_epochs: self.num_unstake_epochs,
        };

        let mut validator_set = BTreeMap::new();

        for (key, bucket) in self.validator_set {
            let stake = Decimal::one();
            let address = EpochManager::create_validator(
                global_node_id.into(),
                key,
                Some(bucket),
                true,
                api,
            )?;
            let validator = Validator { key, stake };
            validator_set.insert(address, validator);
        }

        let current_validator_set = ValidatorSetSubstate {
            epoch: self.initial_epoch,
            validator_set: validator_set.clone(),
        };

        let preparing_validator_set = ValidatorSetSubstate {
            epoch: self.initial_epoch + 1,
            validator_set,
        };

        let mut access_rules = AccessRules::new();
        access_rules.set_method_access_rule(
            AccessRuleKey::Native(NativeFn::EpochManager(EpochManagerFn::NextRound)),
            rule!(require(AuthAddresses::validator_role())),
        );
        access_rules.set_method_access_rule(
            AccessRuleKey::Native(NativeFn::EpochManager(EpochManagerFn::GetCurrentEpoch)),
            rule!(allow_all),
        );
        access_rules.set_method_access_rule(
            AccessRuleKey::Native(NativeFn::EpochManager(EpochManagerFn::CreateValidator)),
            rule!(allow_all),
        );
        let non_fungible_id = NonFungibleId::Bytes(
            scrypto_encode(&PackageIdentifier::Native(NativePackage::EpochManager)).unwrap(),
        );
        let non_fungible_address = NonFungibleAddress::new(PACKAGE_TOKEN, non_fungible_id);
        access_rules.set_method_access_rule(
            AccessRuleKey::Native(NativeFn::EpochManager(EpochManagerFn::UpdateValidator)),
            rule!(require(non_fungible_address)),
        );
        access_rules.set_method_access_rule(
            AccessRuleKey::Native(NativeFn::EpochManager(EpochManagerFn::SetEpoch)),
            rule!(require(AuthAddresses::system_role())), // Set epoch only used for debugging
        );

        api.create_node(
            underlying_node_id,
            RENode::EpochManager(
                epoch_manager,
                current_validator_set,
                preparing_validator_set,
                AccessRulesChainSubstate {
                    access_rules_chain: vec![access_rules],
                },
            ),
        )?;

        api.create_node(
            global_node_id,
            RENode::Global(GlobalAddressSubstate::EpochManager(
                underlying_node_id.into(),
            )),
        )?;

        let component_address: ComponentAddress = global_node_id.into();
        let mut node_refs_to_copy = HashSet::new();
        node_refs_to_copy.insert(global_node_id);

        let update = CallFrameUpdate {
            node_refs_to_copy,
            nodes_to_move: vec![],
        };

        Ok((component_address, update))
    }
}

pub struct EpochManagerGetCurrentEpochExecutable(RENodeId);

impl ExecutableInvocation for EpochManagerGetCurrentEpochInvocation {
    type Exec = EpochManagerGetCurrentEpochExecutable;

    fn resolve<D: ResolverApi>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::EpochManager(EpochManagerFn::GetCurrentEpoch),
            resolved_receiver,
        );
        let executor = EpochManagerGetCurrentEpochExecutable(resolved_receiver.receiver);

        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for EpochManagerGetCurrentEpochExecutable {
    type Output = u64;

    fn execute<Y, W: WasmEngine>(
        self,
        system_api: &mut Y,
    ) -> Result<(u64, CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi,
    {
        let offset = SubstateOffset::EpochManager(EpochManagerOffset::EpochManager);
        let handle = system_api.lock_substate(self.0, offset, LockFlags::read_only())?;
        let substate_ref = system_api.get_ref(handle)?;
        let epoch_manager = substate_ref.epoch_manager();
        Ok((epoch_manager.epoch, CallFrameUpdate::empty()))
    }
}

pub struct EpochManagerNextRoundExecutable {
    node_id: RENodeId,
    round: u64,
}

impl ExecutableInvocation for EpochManagerNextRoundInvocation {
    type Exec = EpochManagerNextRoundExecutable;

    fn resolve<D: ResolverApi>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::EpochManager(EpochManagerFn::NextRound),
            resolved_receiver,
        );
        let executor = EpochManagerNextRoundExecutable {
            node_id: resolved_receiver.receiver,
            round: self.round,
        };

        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for EpochManagerNextRoundExecutable {
    type Output = ();

    fn execute<Y, W: WasmEngine>(
        self,
        system_api: &mut Y,
    ) -> Result<((), CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi,
    {
        let offset = SubstateOffset::EpochManager(EpochManagerOffset::EpochManager);
        let mgr_handle = system_api.lock_substate(self.node_id, offset, LockFlags::MUTABLE)?;
        let mut substate_mut = system_api.get_ref_mut(mgr_handle)?;
        let epoch_manager = substate_mut.epoch_manager();

        if self.round <= epoch_manager.round {
            return Err(RuntimeError::ApplicationError(
                ApplicationError::EpochManagerError(EpochManagerError::InvalidRoundUpdate {
                    from: epoch_manager.round,
                    to: self.round,
                }),
            ));
        }

        if self.round >= epoch_manager.rounds_per_epoch {
            let offset = SubstateOffset::EpochManager(EpochManagerOffset::PreparingValidatorSet);
            let handle = system_api.lock_substate(self.node_id, offset, LockFlags::MUTABLE)?;
            let mut substate_mut = system_api.get_ref_mut(handle)?;
            let preparing_validator_set = substate_mut.validator_set();
            let prepared_epoch = preparing_validator_set.epoch;
            let next_validator_set = preparing_validator_set.validator_set.clone();
            preparing_validator_set.epoch = prepared_epoch + 1;

            let mut substate_mut = system_api.get_ref_mut(mgr_handle)?;
            let epoch_manager = substate_mut.epoch_manager();
            epoch_manager.epoch = prepared_epoch;
            epoch_manager.round = 0;

            let offset = SubstateOffset::EpochManager(EpochManagerOffset::CurrentValidatorSet);
            let handle = system_api.lock_substate(self.node_id, offset, LockFlags::MUTABLE)?;
            let mut substate_mut = system_api.get_ref_mut(handle)?;
            let validator_set = substate_mut.validator_set();
            validator_set.epoch = prepared_epoch;
            validator_set.validator_set = next_validator_set;
        } else {
            epoch_manager.round = self.round;
        }

        Ok(((), CallFrameUpdate::empty()))
    }
}

pub struct EpochManagerSetEpochExecutable(RENodeId, u64);

impl ExecutableInvocation for EpochManagerSetEpochInvocation {
    type Exec = EpochManagerSetEpochExecutable;

    fn resolve<D: ResolverApi>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::EpochManager(EpochManagerFn::SetEpoch),
            resolved_receiver,
        );
        let executor = EpochManagerSetEpochExecutable(resolved_receiver.receiver, self.epoch);

        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for EpochManagerSetEpochExecutable {
    type Output = ();

    fn execute<Y, W: WasmEngine>(
        self,
        system_api: &mut Y,
    ) -> Result<((), CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi,
    {
        let offset = SubstateOffset::EpochManager(EpochManagerOffset::EpochManager);
        let handle = system_api.lock_substate(self.0, offset, LockFlags::MUTABLE)?;
        let mut substate_mut = system_api.get_ref_mut(handle)?;
        substate_mut.epoch_manager().epoch = self.1;
        Ok(((), CallFrameUpdate::empty()))
    }
}

pub struct EpochManagerCreateValidatorExecutable(RENodeId, EcdsaSecp256k1PublicKey);

impl ExecutableInvocation for EpochManagerCreateValidatorInvocation {
    type Exec = EpochManagerCreateValidatorExecutable;

    fn resolve<D: ResolverApi>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;
        call_frame_update.add_ref(RENodeId::Global(GlobalAddress::Resource(RADIX_TOKEN)));

        let actor = ResolvedActor::method(
            NativeFn::EpochManager(EpochManagerFn::CreateValidator),
            resolved_receiver,
        );
        let executor = EpochManagerCreateValidatorExecutable(resolved_receiver.receiver, self.key);

        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for EpochManagerCreateValidatorExecutable {
    type Output = ComponentAddress;

    fn execute<Y, W: WasmEngine>(
        self,
        api: &mut Y,
    ) -> Result<(ComponentAddress, CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi + EngineApi<RuntimeError> + InvokableModel<RuntimeError>,
    {
        let handle = api.lock_substate(
            self.0,
            SubstateOffset::EpochManager(EpochManagerOffset::EpochManager),
            LockFlags::read_only(),
        )?;
        let substate_ref = api.get_ref(handle)?;
        let epoch_manager = substate_ref.epoch_manager();
        let manager = epoch_manager.address;
        let validator_address = EpochManager::create_validator(manager, self.1, None, false, api)?;
        Ok((
            validator_address,
            CallFrameUpdate::copy_ref(RENodeId::Global(GlobalAddress::Component(
                validator_address,
            ))),
        ))
    }
}

pub struct EpochManagerUpdateValidatorExecutable(RENodeId, ComponentAddress, UpdateValidator);

impl ExecutableInvocation for EpochManagerUpdateValidatorInvocation {
    type Exec = EpochManagerUpdateValidatorExecutable;

    fn resolve<D: ResolverApi>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::EpochManager(EpochManagerFn::UpdateValidator),
            resolved_receiver,
        );
        let executor = EpochManagerUpdateValidatorExecutable(
            resolved_receiver.receiver,
            self.validator_address,
            self.update,
        );

        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for EpochManagerUpdateValidatorExecutable {
    type Output = ();

    fn execute<Y, W: WasmEngine>(self, api: &mut Y) -> Result<((), CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi + InvokableModel<RuntimeError>,
    {
        let offset = SubstateOffset::EpochManager(EpochManagerOffset::PreparingValidatorSet);
        let handle = api.lock_substate(self.0, offset, LockFlags::MUTABLE)?;
        let mut substate_ref = api.get_ref_mut(handle)?;
        let validator_set = substate_ref.validator_set();
        match self.2 {
            UpdateValidator::Register(key, stake) => {
                validator_set
                    .validator_set
                    .insert(self.1, Validator { key, stake });
            }
            UpdateValidator::Unregister => {
                validator_set.validator_set.remove(&self.1);
            }
        }

        Ok(((), CallFrameUpdate::empty()))
    }
}

impl EpochManager {
    pub fn create_validator<Y>(
        manager: ComponentAddress,
        key: EcdsaSecp256k1PublicKey,
        bucket: Option<Bucket>,
        is_registered: bool,
        api: &mut Y,
    ) -> Result<ComponentAddress, RuntimeError>
    where
        Y: SystemApi + EngineApi<RuntimeError> + InvokableModel<RuntimeError>,
    {
        let node_id = api.allocate_node_id(RENodeType::Validator)?;
        let global_node_id = api.allocate_node_id(RENodeType::GlobalValidator)?;
        let address: ComponentAddress = global_node_id.into();
        let mut access_rules = AccessRules::new();
        access_rules.set_method_access_rule(
            AccessRuleKey::Native(NativeFn::Validator(ValidatorFn::Register)),
            rule!(require(NonFungibleAddress::from_public_key(&key))),
        );
        access_rules.set_method_access_rule(
            AccessRuleKey::Native(NativeFn::Validator(ValidatorFn::Unregister)),
            rule!(require(NonFungibleAddress::from_public_key(&key))),
        );
        access_rules.set_method_access_rule(
            AccessRuleKey::Native(NativeFn::Validator(ValidatorFn::Stake)),
            rule!(require(NonFungibleAddress::from_public_key(&key))),
        );
        access_rules.set_method_access_rule(
            AccessRuleKey::Native(NativeFn::Validator(ValidatorFn::Unstake)),
            rule!(require(NonFungibleAddress::from_public_key(&key))),
        );
        access_rules.set_method_access_rule(
            AccessRuleKey::Native(NativeFn::Validator(ValidatorFn::ClaimXrd)),
            rule!(allow_all),
        );

        let mut stake_vault = Vault::sys_new(RADIX_TOKEN, api)?;
        if let Some(bucket) = bucket {
            stake_vault.sys_put(bucket, api)?;
        }
        let unstake_vault = Vault::sys_new(RADIX_TOKEN, api)?;

        let mut unstake_token_auth = BTreeMap::new();
        let non_fungible_id = NonFungibleId::Bytes(
            scrypto_encode(&PackageIdentifier::Native(NativePackage::EpochManager)).unwrap(),
        );
        let non_fungible_address = NonFungibleAddress::new(PACKAGE_TOKEN, non_fungible_id);
        unstake_token_auth.insert(
            Mint,
            (
                rule!(require(non_fungible_address.clone())),
                rule!(deny_all),
            ),
        );
        unstake_token_auth.insert(
            Burn,
            (rule!(require(non_fungible_address)), rule!(deny_all)),
        );
        unstake_token_auth.insert(Withdraw, (rule!(allow_all), rule!(deny_all)));
        unstake_token_auth.insert(Deposit, (rule!(allow_all), rule!(deny_all)));

        let unstake_resource_manager = ResourceManager::sys_new_non_fungible(
            NonFungibleIdTypeId::UUID,
            BTreeMap::new(),
            unstake_token_auth,
            api,
        )?;
        let unstake_nft_address = unstake_resource_manager.0;

        let node = RENode::Validator(
            ValidatorSubstate {
                manager,
                key,
                address,
                unstake_nft_address,
                stake_vault_id: stake_vault.0,
                unstake_vault_id: unstake_vault.0,
                is_registered,
            },
            AccessRulesChainSubstate {
                access_rules_chain: vec![access_rules],
            },
        );
        api.create_node(node_id, node)?;
        api.create_node(
            global_node_id,
            RENode::Global(GlobalAddressSubstate::Validator(node_id.into())),
        )?;

        Ok(global_node_id.into())
    }

    pub fn create_auth() -> Vec<MethodAuthorization> {
        vec![MethodAuthorization::Protected(HardAuthRule::ProofRule(
            HardProofRule::Require(HardResourceOrNonFungible::NonFungible(
                AuthAddresses::system_role(),
            )),
        ))]
    }
}
