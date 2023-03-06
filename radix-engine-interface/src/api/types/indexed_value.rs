use crate::api::types::*;
use core::convert::Infallible;
use radix_engine_common::data::scrypto::model::*;
use radix_engine_common::data::scrypto::*;
use sbor::path::SborPathBuf;
use sbor::rust::fmt;
use sbor::rust::prelude::*;
use sbor::traversal::TraversalEvent;
use sbor::*;
use utils::ContextualDisplay;

#[derive(Clone, PartialEq, Eq)]
pub struct IndexedScryptoValue {
    bytes: Vec<u8>,
    global_references: HashSet<RENodeId>,
    owned_nodes: Vec<RENodeId>,
}

impl IndexedScryptoValue {
    fn new(bytes: Vec<u8>) -> Result<Self, DecodeError> {
        let mut traverser = ScryptoTraverser::new(
            &bytes,
            SCRYPTO_SBOR_V1_MAX_DEPTH,
            Some(SCRYPTO_SBOR_V1_PAYLOAD_PREFIX),
            true,
        );
        let mut global_references = HashSet::<RENodeId>::new();
        let mut owned_nodes = Vec::<RENodeId>::new();
        loop {
            let event = traverser.next_event();
            match event.event {
                TraversalEvent::PayloadPrefix => {}
                TraversalEvent::ContainerStart(_) => {}
                TraversalEvent::ContainerEnd(_) => {}
                TraversalEvent::TerminalValue(r) => {
                    if let traversal::TerminalValueRef::Custom(c) = r {
                        match c.0 {
                            ScryptoCustomValue::Address(a) => {
                                global_references.insert(a.into());
                            }
                            ScryptoCustomValue::Own(o) => {
                                owned_nodes.push(match o {
                                    Own::Bucket(id)
                                    | Own::Proof(id)
                                    | Own::Vault(id)
                                    | Own::Object(id) => RENodeId::Object(id),
                                    Own::KeyValueStore(id) => RENodeId::KeyValueStore(id),
                                });
                            }
                            ScryptoCustomValue::Decimal(_)
                            | ScryptoCustomValue::PreciseDecimal(_)
                            | ScryptoCustomValue::NonFungibleLocalId(_) => {}
                        }
                    }
                }
                TraversalEvent::TerminalValueBatch(_) => {}
                TraversalEvent::End => {
                    break;
                }
                TraversalEvent::DecodeError(e) => {
                    return Err(e);
                }
            }
        }

        Ok(Self {
            bytes,
            global_references,
            owned_nodes,
        })
    }

    pub fn unit() -> Self {
        Self::from_typed(&())
    }

    /// Converts a rust value into `IndexedScryptoValue`, assuming it follows RE semantics.
    pub fn from_typed<T: ScryptoEncode + ?Sized>(value: &T) -> Self {
        let bytes = scrypto_encode(value).expect("Failed to encode trusted Rust value");
        Self::new(bytes).expect("Failed to index trusted Rust value")
    }

    pub fn from_slice(slice: &[u8]) -> Result<Self, DecodeError> {
        Self::new(slice.to_vec())
    }

    pub fn from_vec(vec: Vec<u8>) -> Result<Self, DecodeError> {
        Self::new(vec)
    }

    pub fn from_scrypto_value(value: ScryptoValue) -> Self {
        let bytes = scrypto_encode(&value).expect("Failed to encode trusted ScryptoValue");
        Self::new(bytes).expect("Failed to index trusted ScryptoValue")
    }

    pub fn to_scrypto_value(&self) -> ScryptoValue {
        scrypto_decode(&self.bytes).expect("Failed to decode bytes in IndexedScryptoValue")
    }

    pub fn as_typed<T: ScryptoDecode>(&self) -> Result<T, DecodeError> {
        scrypto_decode(&self.bytes)
    }

    pub fn as_slice(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    pub fn global_references(&self) -> &HashSet<RENodeId> {
        &self.global_references
    }

    pub fn owned_node_ids(&self) -> &Vec<RENodeId> {
        &self.owned_nodes
    }

    pub fn unpack(self) -> (Vec<u8>, Vec<RENodeId>, HashSet<RENodeId>) {
        (self.bytes, self.owned_nodes, self.global_references)
    }
}

impl Into<Vec<u8>> for IndexedScryptoValue {
    fn into(self) -> Vec<u8> {
        self.bytes
    }
}

impl fmt::Debug for IndexedScryptoValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_scrypto_value(
            f,
            &self.to_scrypto_value(),
            &ScryptoValueDisplayContext::no_context(),
        )
    }
}

impl<'a> ContextualDisplay<ScryptoValueDisplayContext<'a>> for IndexedScryptoValue {
    type Error = fmt::Error;

    fn contextual_format<F: fmt::Write>(
        &self,
        f: &mut F,
        context: &ScryptoValueDisplayContext<'a>,
    ) -> Result<(), Self::Error> {
        format_scrypto_value(f, &self.to_scrypto_value(), context)
    }
}

pub struct ScryptoValueVisitor {
    pub global_references: HashSet<RENodeId>,
    pub owned_nodes: Vec<RENodeId>,
}

impl ScryptoValueVisitor {
    pub fn new() -> Self {
        Self {
            global_references: HashSet::new(),
            owned_nodes: Vec::new(),
        }
    }
}

impl ValueVisitor<ScryptoCustomValueKind, ScryptoCustomValue> for ScryptoValueVisitor {
    type Err = Infallible;

    fn visit(
        &mut self,
        _path: &mut SborPathBuf,
        value: &ScryptoCustomValue,
    ) -> Result<(), Self::Err> {
        match value {
            ScryptoCustomValue::Address(value) => {
                self.global_references.insert(value.clone().into());
            }
            ScryptoCustomValue::Own(value) => {
                match value {
                    Own::Bucket(object_id) => {
                        self.owned_nodes.push(RENodeId::Object(*object_id));
                    }
                    Own::Proof(proof_id) => {
                        self.owned_nodes.push(RENodeId::Object(*proof_id));
                    }
                    Own::Vault(vault_id) => self.owned_nodes.push(RENodeId::Object(*vault_id)),
                    Own::Object(component_id) => {
                        self.owned_nodes.push(RENodeId::Object(*component_id))
                    }
                    Own::KeyValueStore(kv_store_id) => {
                        self.owned_nodes.push(RENodeId::KeyValueStore(*kv_store_id))
                    }
                };
            }

            ScryptoCustomValue::Decimal(_)
            | ScryptoCustomValue::PreciseDecimal(_)
            | ScryptoCustomValue::NonFungibleLocalId(_) => {
                // no-op
            }
        }
        Ok(())
    }
}
