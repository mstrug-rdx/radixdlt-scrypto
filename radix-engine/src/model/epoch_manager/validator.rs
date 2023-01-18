use crate::engine::{
    deref_and_update, ApplicationError, CallFrameUpdate, ExecutableInvocation, Executor, LockFlags,
    ResolvedActor, ResolverApi, RuntimeError, SystemApi,
};
use crate::types::*;
use crate::wasm::WasmEngine;
use native_sdk::resource::{ResourceManager, SysBucket, Vault};
use radix_engine_interface::api::api::{EngineApi, InvokableModel};
use radix_engine_interface::api::types::{GlobalAddress, NativeFn, RENodeId, SubstateOffset};
use radix_engine_interface::model::*;

#[derive(Debug, Clone, PartialEq, Eq, ScryptoCategorize, ScryptoEncode, ScryptoDecode)]
pub struct ValidatorSubstate {
    pub manager: ComponentAddress,
    pub address: ComponentAddress,
    pub unstake_nft_address: ResourceAddress,
    pub key: EcdsaSecp256k1PublicKey,
    pub stake_vault_id: VaultId,
    pub unstake_vault_id: VaultId,
    pub is_registered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, ScryptoCategorize, ScryptoEncode, ScryptoDecode)]
pub enum ValidatorError {
    InvalidClaimResource,
    EpochUnlockHasNotOccurredYet,
}

pub struct ValidatorRegisterExecutable(RENodeId);

impl ExecutableInvocation for ValidatorRegisterInvocation {
    type Exec = ValidatorRegisterExecutable;

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
            NativeFn::Validator(ValidatorFn::Register),
            resolved_receiver,
        );
        let executor = ValidatorRegisterExecutable(resolved_receiver.receiver);
        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for ValidatorRegisterExecutable {
    type Output = ();

    fn execute<Y, W: WasmEngine>(self, api: &mut Y) -> Result<((), CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi + EngineApi<RuntimeError> + InvokableModel<RuntimeError>,
    {
        let offset = SubstateOffset::Validator(ValidatorOffset::Validator);
        let handle = api.lock_substate(self.0, offset.clone(), LockFlags::MUTABLE)?;

        // Update state
        {
            let mut substate = api.get_ref_mut(handle)?;
            let validator = substate.validator();

            if validator.is_registered {
                return Ok(((), CallFrameUpdate::empty()));
            }

            validator.is_registered = true;
        }

        // Update EpochManager
        {
            let substate = api.get_ref(handle)?;
            let validator = substate.validator();
            let stake_vault = Vault(validator.stake_vault_id);
            let stake_amount = stake_vault.sys_amount(api)?;
            if stake_amount.is_positive() {
                let substate = api.get_ref(handle)?;
                let validator = substate.validator();
                let invocation = EpochManagerUpdateValidatorInvocation {
                    receiver: validator.manager,
                    validator_address: validator.address,
                    update: UpdateValidator::Register(validator.key, stake_amount),
                };
                api.invoke(invocation)?;
            }
        }

        Ok(((), CallFrameUpdate::empty()))
    }
}

pub struct ValidatorUnregisterExecutable(RENodeId);

impl ExecutableInvocation for ValidatorUnregisterInvocation {
    type Exec = ValidatorUnregisterExecutable;

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
            NativeFn::Validator(ValidatorFn::Unregister),
            resolved_receiver,
        );
        let executor = ValidatorUnregisterExecutable(resolved_receiver.receiver);
        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for ValidatorUnregisterExecutable {
    type Output = ();

    fn execute<Y, W: WasmEngine>(self, api: &mut Y) -> Result<((), CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi + InvokableModel<RuntimeError>,
    {
        let offset = SubstateOffset::Validator(ValidatorOffset::Validator);
        let handle = api.lock_substate(self.0, offset.clone(), LockFlags::MUTABLE)?;

        // Update state
        {
            let mut substate = api.get_ref_mut(handle)?;
            let validator = substate.validator();
            if !validator.is_registered {
                return Ok(((), CallFrameUpdate::empty()));
            }
            validator.is_registered = false;
        }

        // Update EpochManager
        {
            let mut substate = api.get_ref_mut(handle)?;
            let validator = substate.validator();
            let invocation = EpochManagerUpdateValidatorInvocation {
                receiver: validator.manager,
                validator_address: validator.address,
                update: UpdateValidator::Unregister,
            };
            api.invoke(invocation)?;
        }

        Ok(((), CallFrameUpdate::empty()))
    }
}

pub struct ValidatorStakeExecutable(RENodeId, Bucket);

impl ExecutableInvocation for ValidatorStakeInvocation {
    type Exec = ValidatorStakeExecutable;

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
        call_frame_update
            .nodes_to_move
            .push(RENodeId::Bucket(self.stake.0));

        let actor =
            ResolvedActor::method(NativeFn::Validator(ValidatorFn::Stake), resolved_receiver);
        let executor = ValidatorStakeExecutable(resolved_receiver.receiver, self.stake);
        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for ValidatorStakeExecutable {
    type Output = ();

    fn execute<Y, W: WasmEngine>(self, api: &mut Y) -> Result<((), CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi + EngineApi<RuntimeError> + InvokableModel<RuntimeError>,
    {
        let offset = SubstateOffset::Validator(ValidatorOffset::Validator);
        let handle = api.lock_substate(self.0, offset, LockFlags::read_only())?;

        // Stake
        {
            let substate = api.get_ref(handle)?;
            let validator = substate.validator();
            let mut xrd_vault = Vault(validator.stake_vault_id);
            xrd_vault.sys_put(self.1, api)?;
        }

        // Update EpochManager
        {
            let substate = api.get_ref(handle)?;
            let validator = substate.validator();
            if validator.is_registered {
                let receiver = validator.manager;
                let key = validator.key;
                let validator_address = validator.address;
                let xrd_vault = Vault(validator.stake_vault_id);
                let xrd_amount = xrd_vault.sys_amount(api)?;
                let invocation = EpochManagerUpdateValidatorInvocation {
                    receiver,
                    validator_address,
                    update: UpdateValidator::Register(key, xrd_amount),
                };
                api.invoke(invocation)?;
            }
        }

        Ok(((), CallFrameUpdate::empty()))
    }
}

pub struct ValidatorUnstakeExecutable(RENodeId, Decimal);

impl ExecutableInvocation for ValidatorUnstakeInvocation {
    type Exec = ValidatorUnstakeExecutable;

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

        let actor =
            ResolvedActor::method(NativeFn::Validator(ValidatorFn::Unstake), resolved_receiver);
        let executor = ValidatorUnstakeExecutable(resolved_receiver.receiver, self.amount);
        Ok((actor, call_frame_update, executor))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, ScryptoCategorize, ScryptoEncode, ScryptoDecode)]
pub struct UnstakeData {
    epoch_unlocked: u64,
    amount: Decimal,
}

impl Executor for ValidatorUnstakeExecutable {
    type Output = Bucket;

    fn execute<Y, W: WasmEngine>(
        self,
        api: &mut Y,
    ) -> Result<(Bucket, CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi + EngineApi<RuntimeError> + InvokableModel<RuntimeError>,
    {
        let offset = SubstateOffset::Validator(ValidatorOffset::Validator);
        let handle = api.lock_substate(self.0, offset, LockFlags::read_only())?;

        // Unstake
        let unstake_bucket = {
            let substate = api.get_ref(handle)?;
            let validator = substate.validator();

            let manager = validator.manager;
            let mut stake_vault = Vault(validator.stake_vault_id);
            let mut unstake_vault = Vault(validator.unstake_vault_id);
            let mut nft_resman = ResourceManager(validator.unstake_nft_address);
            let manager_handle = api.lock_substate(
                RENodeId::Global(GlobalAddress::Component(manager)),
                SubstateOffset::EpochManager(EpochManagerOffset::EpochManager),
                LockFlags::read_only(),
            )?;
            let manager_substate = api.get_ref(manager_handle)?;
            let epoch_manager = manager_substate.epoch_manager();
            let current_epoch = epoch_manager.epoch;
            let epoch_unlocked = current_epoch + epoch_manager.num_unstake_epochs;
            api.drop_lock(manager_handle)?;

            let data = UnstakeData {
                epoch_unlocked,
                amount: self.1,
            };

            let bucket = stake_vault.sys_take(self.1, api)?;
            unstake_vault.sys_put(bucket, api)?;
            nft_resman.mint_non_fungible_uuid(data, api)?
        };

        // Update Epoch Manager
        {
            let substate = api.get_ref(handle)?;
            let validator = substate.validator();
            let stake_vault = Vault(validator.stake_vault_id);
            if validator.is_registered {
                let stake_amount = stake_vault.sys_amount(api)?;
                let substate = api.get_ref(handle)?;
                let validator = substate.validator();
                let update = if stake_amount.is_zero() {
                    UpdateValidator::Unregister
                } else {
                    UpdateValidator::Register(validator.key, stake_amount)
                };

                let invocation = EpochManagerUpdateValidatorInvocation {
                    receiver: validator.manager,
                    validator_address: validator.address,
                    update,
                };
                api.invoke(invocation)?;
            }
        };

        let update = CallFrameUpdate::move_node(RENodeId::Bucket(unstake_bucket.0));
        Ok((unstake_bucket, update))
    }
}

pub struct ValidatorClaimXrdExecutable(RENodeId, Bucket);

impl ExecutableInvocation for ValidatorClaimXrdInvocation {
    type Exec = ValidatorClaimXrdExecutable;

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
        call_frame_update
            .nodes_to_move
            .push(RENodeId::Bucket(self.bucket.0));

        let actor = ResolvedActor::method(
            NativeFn::Validator(ValidatorFn::ClaimXrd),
            resolved_receiver,
        );
        let executor = ValidatorClaimXrdExecutable(resolved_receiver.receiver, self.bucket);
        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for ValidatorClaimXrdExecutable {
    type Output = Bucket;

    fn execute<Y, W: WasmEngine>(
        self,
        api: &mut Y,
    ) -> Result<(Bucket, CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi + EngineApi<RuntimeError> + InvokableModel<RuntimeError>,
    {
        let offset = SubstateOffset::Validator(ValidatorOffset::Validator);
        let handle = api.lock_substate(self.0, offset, LockFlags::read_only())?;
        let substate = api.get_ref(handle)?;
        let validator = substate.validator();
        let mut nft_resman = ResourceManager(validator.unstake_nft_address);
        let resource_address = validator.unstake_nft_address;
        let manager = validator.manager;
        let mut unstake_vault = Vault(validator.unstake_vault_id);

        // TODO: Move this check into a more appropriate place
        let bucket = Bucket(self.1 .0);
        if !resource_address.eq(&bucket.sys_resource_address(api)?) {
            return Err(RuntimeError::ApplicationError(
                ApplicationError::ValidatorError(ValidatorError::InvalidClaimResource),
            ));
        }

        let current_epoch = {
            let mgr_handle = api.lock_substate(
                RENodeId::Global(GlobalAddress::Component(manager)),
                SubstateOffset::EpochManager(EpochManagerOffset::EpochManager),
                LockFlags::read_only(),
            )?;
            let mgr_substate = api.get_ref(mgr_handle)?;
            let epoch = mgr_substate.epoch_manager().epoch;
            api.drop_lock(mgr_handle)?;
            epoch
        };

        let mut unstake_amount = Decimal::zero();

        for id in bucket.sys_non_fungible_ids(api)? {
            let data: UnstakeData = nft_resman.get_non_fungible_data(id, api)?;
            if current_epoch < data.epoch_unlocked {
                return Err(RuntimeError::ApplicationError(
                    ApplicationError::ValidatorError(ValidatorError::EpochUnlockHasNotOccurredYet),
                ));
            }
            unstake_amount += data.amount;
        }
        nft_resman.burn(bucket, api)?;

        let claimed_bucket = unstake_vault.sys_take(unstake_amount, api)?;
        let update = CallFrameUpdate::move_node(RENodeId::Bucket(claimed_bucket.0));
        Ok((claimed_bucket, update))
    }
}
