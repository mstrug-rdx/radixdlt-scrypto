use sbor::rust::borrow::ToOwned;
use sbor::rust::string::String;
use sbor::rust::vec::Vec;
use sbor::*;
use scrypto::buffer::scrypto_decode;
use scrypto::component::ComponentAddAccessCheckInput;
use scrypto::engine::types::*;
use scrypto::prelude::ComponentGlobalizeInput;
use scrypto::resource::AccessRules;
use scrypto::values::*;

use crate::engine::SystemApi;
use crate::fee::CostUnitCounterError;
use crate::model::{convert, MethodAuthorization};
use crate::wasm::{WasmEngine, WasmInstance};

#[derive(Debug, Clone, PartialEq)]
pub enum ComponentError {
    InvalidRequestData(DecodeError),
    BlueprintFunctionDoesNotExist(String),
    MethodNotFound,
    CostingError(CostUnitCounterError),
}

/// A component is an instance of blueprint.
#[derive(Debug, Clone, TypeId, Encode, Decode)]
pub struct Component {
    package_address: PackageAddress,
    blueprint_name: String,
    access_rules: Vec<AccessRules>,
    state: Vec<u8>,
}

impl Component {
    pub fn new(
        package_address: PackageAddress,
        blueprint_name: String,
        access_rules: Vec<AccessRules>,
        state: Vec<u8>,
    ) -> Self {
        Self {
            package_address,
            blueprint_name,
            access_rules,
            state,
        }
    }

    pub fn method_authorization(
        &self,
        schema: &Type,
        method_name: &str,
    ) -> Vec<MethodAuthorization> {
        let data = ScryptoValue::from_slice(&self.state).unwrap();

        let mut authorizations = Vec::new();
        for auth in &self.access_rules {
            let method_auth = auth.get(method_name);
            let authorization = convert(schema, &data.dom, method_auth);
            authorizations.push(authorization);
        }

        authorizations
    }

    pub fn info(&self) -> (PackageAddress, String) {
        (self.package_address.clone(), self.blueprint_name.clone())
    }

    pub fn authorization(&self) -> &[AccessRules] {
        &self.access_rules
    }

    pub fn package_address(&self) -> PackageAddress {
        self.package_address.clone()
    }

    pub fn blueprint_name(&self) -> &str {
        &self.blueprint_name
    }

    pub fn state(&self) -> &[u8] {
        &self.state
    }

    pub fn set_state(&mut self, new_state: Vec<u8>) {
        self.state = new_state;
    }

    pub fn main<'p, Y: SystemApi<'p, W, I>, W: WasmEngine<I>, I: WasmInstance>(
        value_id: ValueId,
        fn_ident: &str,
        arg: ScryptoValue,
        system_api: &mut Y,
    ) -> Result<ScryptoValue, ComponentError> {
        let rtn = match fn_ident {
            "add_access_check" => {
                let input: ComponentAddAccessCheckInput =
                    scrypto_decode(&arg.raw).map_err(|e| ComponentError::InvalidRequestData(e))?;

                // Abi checks
                {
                    let component_ref = system_api
                        .borrow_value(&value_id)
                        .map_err(ComponentError::CostingError)?;
                    let component = component_ref.component();
                    let component_name = component.blueprint_name().to_owned();
                    let package_id = ValueId::Package(component.package_address.clone());
                    drop(component);
                    drop(component_ref);
                    let package_ref = system_api
                        .borrow_value(&package_id)
                        .map_err(ComponentError::CostingError)?;
                    let package = package_ref.package();
                    let blueprint_abi = package.blueprint_abi(&component_name).unwrap();
                    for (func_name, _) in input.access_rules.iter() {
                        if !blueprint_abi.contains_fn(func_name.as_str()) {
                            return Err(ComponentError::BlueprintFunctionDoesNotExist(
                                func_name.to_string(),
                            ));
                        }
                    }
                }

                let mut ref_mut = system_api
                    .borrow_value_mut(&value_id)
                    .map_err(ComponentError::CostingError)?;
                let component = ref_mut.component();
                component.access_rules.push(input.access_rules);
                system_api
                    .return_value_mut(value_id, ref_mut)
                    .map_err(ComponentError::CostingError)?;

                Ok(ScryptoValue::from_typed(&()))
            }
            "globalize" => {
                let _: ComponentGlobalizeInput =
                    scrypto_decode(&arg.raw).map_err(|e| ComponentError::InvalidRequestData(e))?;

                system_api
                    .globalize_value(&value_id)
                    .map_err(ComponentError::CostingError)?;
                Ok(ScryptoValue::from_typed(&()))
            }
            _ => Err(ComponentError::MethodNotFound),
        }?;

        Ok(rtn)
    }

    pub fn main_consume<'p, Y: SystemApi<'p, W, I>, W: WasmEngine<I>, I: WasmInstance>(
        value_id: ValueId,
        fn_ident: &str,
        arg: ScryptoValue,
        system_api: &mut Y,
    ) -> Result<ScryptoValue, ComponentError> {
        let rtn = match fn_ident {
            "globalize" => {
                let _: ComponentGlobalizeInput =
                    scrypto_decode(&arg.raw).map_err(|e| ComponentError::InvalidRequestData(e))?;

                system_api
                    .globalize_value(&value_id)
                    .map_err(ComponentError::CostingError)?;
                Ok(ScryptoValue::from_typed(&()))
            }
            _ => Err(ComponentError::MethodNotFound),
        }?;

        Ok(rtn)
    }
}
