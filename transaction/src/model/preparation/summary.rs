use crate::internal_prelude::*;

/// This is created as part of the preparation of a transaction, during decoding.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Summary {
    /// This is an approximation of the encoded length of the underlying items contained within the item,
    /// which needs to be **deterministic** from the information in the hash.
    /// Not necessarily the _actual_ length of the payload.
    pub effective_length: usize,
    /// The total number of bytes which were hashed.
    pub total_bytes_hashed: usize,
    /// The hash by which this payload is identified.
    /// This might be a hash of the payload itself, or of a composite hash of some concatenation of other hashes/values.
    pub hash: Hash,
}

pub trait HasSummary {
    fn get_summary(&self) -> &Summary;
}
