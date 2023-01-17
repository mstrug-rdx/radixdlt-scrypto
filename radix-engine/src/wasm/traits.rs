use super::InstrumentedCode;
use crate::model::InvokeError;
use crate::wasm::errors::*;
use radix_engine_interface::api::types::LockHandle;
use radix_engine_interface::api::wasm::*;
use sbor::rust::boxed::Box;
use sbor::rust::vec::Vec;

/// Represents the runtime that can be invoked by Scrypto modules.
pub trait WasmRuntime {
    fn allocate_buffer(&mut self, buffer: Vec<u8>) -> Result<Buffer, InvokeError<WasmShimError>>;

    fn consume_buffer(
        &mut self,
        buffer_id: BufferId,
    ) -> Result<Vec<u8>, InvokeError<WasmShimError>>;

    fn invoke_method(
        &mut self,
        receiver: Vec<u8>,
        ident: Vec<u8>,
        args: Vec<u8>,
    ) -> Result<Buffer, InvokeError<WasmShimError>>;

    fn invoke(&mut self, invocation: Vec<u8>) -> Result<Buffer, InvokeError<WasmShimError>>;

    fn create_node(&mut self, node: Vec<u8>) -> Result<Buffer, InvokeError<WasmShimError>>;

    fn get_visible_nodes(&mut self) -> Result<Buffer, InvokeError<WasmShimError>>;

    fn drop_node(&mut self, node_id: Vec<u8>) -> Result<(), InvokeError<WasmShimError>>;

    fn lock_substate(
        &mut self,
        node_id: Vec<u8>,
        offset: Vec<u8>,
        mutable: bool,
    ) -> Result<LockHandle, InvokeError<WasmShimError>>;

    fn read_substate(&mut self, handle: LockHandle) -> Result<Buffer, InvokeError<WasmShimError>>;

    fn write_substate(
        &mut self,
        handle: LockHandle,
        data: Vec<u8>,
    ) -> Result<(), InvokeError<WasmShimError>>;

    fn unlock_substate(&mut self, handle: LockHandle) -> Result<(), InvokeError<WasmShimError>>;

    fn get_actor(&mut self) -> Result<Buffer, InvokeError<WasmShimError>>;

    fn consume_cost_units(&mut self, n: u32) -> Result<(), InvokeError<WasmShimError>>;
}

/// Represents an instantiated, invokable Scrypto module.
pub trait WasmInstance {
    /// Invokes an export defined in this module.
    ///
    /// The expected signature is as follows:
    /// - The argument list is variable number of U32/U32
    /// - The return data is a U64/I64, which encapsulates a pointer and a length.
    fn invoke_export<'r>(
        &mut self,
        func_name: &str,
        args: Vec<u32>,
        runtime: &mut Box<dyn WasmRuntime + 'r>,
    ) -> Result<Vec<u8>, InvokeError<WasmShimError>>;
}

/// A Scrypto WASM engine validates, instruments and runs Scrypto modules.
pub trait WasmEngine {
    type WasmInstance: WasmInstance;

    /// Instantiate a Scrypto module.
    fn instantiate(&self, instrumented_code: &InstrumentedCode) -> Self::WasmInstance;
}
