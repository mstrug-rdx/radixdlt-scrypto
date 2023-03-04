use crate::blueprints::epoch_manager::Validator;
use crate::types::*;

#[derive(ScryptoSbor)]
pub struct RoundChangeEvent {
    pub round: u64,
}

#[derive(ScryptoSbor)]
pub struct EpochChangeEvent {
    pub epoch: u64,
    pub validators: BTreeMap<ComponentAddress, Validator>,
}
