use crate::api::ObjectModuleId;
use crate::blueprints::package::TypePointer;
use crate::ScryptoSbor;
use radix_engine_common::address::AddressDisplayContext;
use radix_engine_common::types::NodeId;
use sbor::rust::fmt;
use sbor::rust::string::String;
use utils::ContextualDisplay;

/// Identifies a specific event schema emitter by some emitter RENode.
///
/// This type is an identifier uses to identify the schema of events emitted by an RENode of some
/// [`NodeId`]. With this identifier, the schema for an event can be queried, obtained, and with
/// it, the SBOR encoded event data can be decoded and understood.
///
/// It is important to note that application events are always emitted by an RENode, meaning that
/// there is always an emitter of some [`NodeId`].
#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub struct EventTypeIdentifier(pub Emitter, pub TypePointer);

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub enum Emitter {
    // (Node id, module id, blueprint name)
    Function(NodeId, ObjectModuleId, String),
    // (Node id, module id)
    Method(NodeId, ObjectModuleId),
}

impl<'a> ContextualDisplay<AddressDisplayContext<'a>> for Emitter {
    type Error = fmt::Error;

    fn contextual_format<F: fmt::Write>(
        &self,
        f: &mut F,
        context: &AddressDisplayContext<'a>,
    ) -> Result<(), Self::Error> {
        match self {
            Self::Function(node_id, module_id, blueprint_name) => {
                write!(
                    f,
                    "Function {{ node: {}, module_id: {:?}, blueprint_name: {} }}",
                    node_id.display(*context),
                    module_id,
                    blueprint_name
                )
            }
            Self::Method(node_id, module_id) => {
                write!(
                    f,
                    "Method {{ node: {}, module_id: {:?} }}",
                    node_id.display(*context),
                    module_id,
                )
            }
        }
    }
}
