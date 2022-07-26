use sbor::rust::any::Any;
use sbor::rust::ops::{Deref, DerefMut};
use sbor::rust::boxed::Box;
use sbor::{Decode, Encode};
use crate::buffer::*;
use crate::component::package::Package;
use crate::component::*;
use crate::core::{ComponentOffset, DataAddress, SNodeRef};
use crate::engine::{api::*, call_engine};
use sbor::rust::borrow::ToOwned;
use sbor::rust::collections::*;
use sbor::rust::string::ToString;

pub struct ComponentDataRef<'a, V: Encode> {
    value: &'a V,
}

impl<'a, V: Encode> Deref for ComponentDataRef<'a, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

pub struct ComponentDataRefMut<'a, V: Encode> {
    address: ComponentAddress,
    value: &'a mut V,
}

impl<'a, V: Encode> Drop for ComponentDataRefMut<'a, V> {
    fn drop(&mut self) {
        let address = DataAddress::Component(self.address.clone(), ComponentOffset::State);
        let bytes = scrypto_encode(self.value);
        let input = ::scrypto::engine::api::RadixEngineInput::WriteData(address, bytes);
        let _: () = ::scrypto::engine::call_engine(input);
    }
}

impl<'a, V: Encode> Deref for ComponentDataRefMut<'a, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'a, V: Encode> DerefMut for ComponentDataRefMut<'a, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

pub struct ComponentDataSystem {
    data: HashMap<ComponentAddress, Box<dyn Any>>
}

impl ComponentDataSystem {
    pub fn new() -> Self {
        Self {
            data: HashMap::new()
        }
    }

    /// Returns a reference to component data
    pub fn get_data_mut<V: 'static + Encode + Decode>(&mut self, component_address: &ComponentAddress) -> ComponentDataRefMut<V> {
        if !self.data.contains_key(component_address) {
            let address = DataAddress::Component(*component_address, ComponentOffset::State);
            let input = ::scrypto::engine::api::RadixEngineInput::ReadData(address);
            let value: V = call_engine(input);
            self.data.insert(*component_address, Box::new(value));
        }

        let value = self.data.get_mut(component_address).unwrap().downcast_mut().unwrap();
        ComponentDataRefMut {
            address: *component_address,
            value
        }
    }

    pub fn get_data<V: 'static + Encode + Decode>(&mut self, component_address: &ComponentAddress) -> ComponentDataRef<V> {
        if !self.data.contains_key(component_address) {
            let address = DataAddress::Component(*component_address, ComponentOffset::State);
            let input = ::scrypto::engine::api::RadixEngineInput::ReadData(address);
            let value: V = call_engine(input);
            self.data.insert(*component_address, Box::new(value));
        }

        let value = self.data.get(component_address).unwrap().downcast_ref().unwrap();
        ComponentDataRef {
            value
        }
    }
}

/// Represents the Radix Engine component subsystem.
///
/// Notes:
/// - No mutability semantics are enforced
/// - It's not thread safe
///
/// TODO: research if need to introduce `&` and `&mut` for packages and components.
/// TODO: add mutex/lock for non-WebAssembly target
pub struct ComponentSystem {
    packages: HashMap<PackageAddress, BorrowedPackage>,
    components: HashMap<ComponentAddress, Component>,
}

impl ComponentSystem {
    /// Creates a component system.
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
            components: HashMap::new(),
        }
    }

    /// Returns a reference to a package.
    pub fn get_package(&mut self, package_address: PackageAddress) -> &BorrowedPackage {
        self.packages
            .entry(package_address)
            .or_insert(BorrowedPackage(package_address))
    }

    /// Returns a reference to a component.
    pub fn get_component(&mut self, component_address: ComponentAddress) -> &Component {
        self.components
            .entry(component_address)
            .or_insert(Component(component_address))
    }

    /// Publishes a package.
    pub fn publish_package(&mut self, package: Package) -> PackageAddress {
        let input = RadixEngineInput::InvokeSNode(
            SNodeRef::PackageStatic,
            "publish".to_string(),
            scrypto_encode(&PackagePublishInput { package }),
        );
        call_engine(input)
    }

    /// Instantiates a component.
    pub fn create_component<T: ComponentState<C>, C: LocalComponent>(
        &self,
        blueprint_name: &str,
        state: T,
    ) -> Component {
        let input =
            RadixEngineInput::CreateComponent(blueprint_name.to_owned(), scrypto_encode(&state));
        let component_address: ComponentAddress = call_engine(input);

        Component(component_address)
    }
}

static mut COMPONENT_SYSTEM: Option<ComponentSystem> = None;

/// Initializes component subsystem.
pub fn init_component_system(system: ComponentSystem) {
    unsafe { COMPONENT_SYSTEM = Some(system) }
}

/// Returns the component subsystem.
pub fn component_system() -> &'static mut ComponentSystem {
    unsafe { COMPONENT_SYSTEM.as_mut().unwrap() }
}

/// This macro creates a `&Package` from a `PackageAddress` via the
/// Radix Engine component subsystem.
#[macro_export]
macro_rules! borrow_package {
    ($id:expr) => {
        component_system().get_package($id)
    };
}

/// This macro converts a `ComponentAddress` into a `&Component` via the
/// Radix Engine component subsystem.
#[macro_export]
macro_rules! borrow_component {
    ($id:expr) => {
        component_system().get_component($id)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_macro() {
        init_component_system(ComponentSystem::new());

        let component = borrow_component!(ComponentAddress([0u8; 27]));
        let component_same_id = borrow_component!(ComponentAddress([0u8; 27]));
        let component_different_id = borrow_component!(ComponentAddress([1u8; 27]));

        assert_eq!(ComponentAddress([0u8; 27]), component.0);
        assert_eq!(ComponentAddress([0u8; 27]), component_same_id.0);
        assert_eq!(ComponentAddress([1u8; 27]), component_different_id.0);
    }

    #[test]
    fn test_package_macro() {
        init_component_system(ComponentSystem::new());

        let package = borrow_package!(PackageAddress([0u8; 27]));
        let package_same_id = borrow_package!(PackageAddress([0u8; 27]));
        let package_different_id = borrow_package!(PackageAddress([1u8; 27]));

        assert_eq!(PackageAddress([0u8; 27]), package.0);
        assert_eq!(PackageAddress([0u8; 27]), package_same_id.0);
        assert_eq!(PackageAddress([1u8; 27]), package_different_id.0);
    }
}
