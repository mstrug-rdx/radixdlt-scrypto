use hashbrown::{HashMap, HashSet};
use scrypto::types::*;
use scrypto::utils::*;

use crate::ledger::*;
use crate::model::*;

/// Represents the transaction execution runtime, one per transaction.
/// A runtime is shared by a chain of processes, created during the execution of the transaction.
pub struct Runtime<T: Ledger> {
    tx_hash: Hash,
    ledger: T,
    component_counter: u32,
    bucket_counter: u32,
    logs: Vec<(Level, String)>,
    blueprints: HashMap<Address, Blueprint>,
    components: HashMap<Address, Component>,
    accounts: HashMap<Address, Account>,
    resources: HashMap<Address, Resource>,
    updated_blueprints: HashSet<Address>,
    updated_components: HashSet<Address>,
    updated_accounts: HashSet<Address>,
    updated_resources: HashSet<Address>,
}

impl<T: Ledger> Runtime<T> {
    pub fn new(tx_hash: Hash, ledger: T) -> Self {
        Self {
            tx_hash,
            ledger,
            component_counter: 0,
            bucket_counter: 0,
            logs: Vec::new(),
            blueprints: HashMap::new(),
            components: HashMap::new(),
            accounts: HashMap::new(),
            resources: HashMap::new(),
            updated_blueprints: HashSet::new(),
            updated_components: HashSet::new(),
            updated_accounts: HashSet::new(),
            updated_resources: HashSet::new(),
        }
    }

    pub fn log(&mut self, level: Level, message: String) {
        self.logs.push((level, message));
    }

    /// Returns an immutable reference to a blueprint, if exists.
    pub fn get_blueprint(&mut self, address: Address) -> Option<&Blueprint> {
        if self.blueprints.contains_key(&address) {
            return self.blueprints.get(&address);
        }

        if let Some(blueprint) = self.ledger.get_blueprint(address) {
            self.blueprints.insert(address, blueprint);
            self.blueprints.get(&address)
        } else {
            None
        }
    }

    /// Returns a mutable reference to a blueprint, if exists.
    #[allow(dead_code)]
    pub fn get_blueprint_mut(&mut self, address: Address) -> Option<&mut Blueprint> {
        self.updated_blueprints.insert(address);

        if self.blueprints.contains_key(&address) {
            return self.blueprints.get_mut(&address);
        }

        if let Some(blueprint) = self.ledger.get_blueprint(address) {
            self.blueprints.insert(address, blueprint);
            self.blueprints.get_mut(&address)
        } else {
            None
        }
    }

    /// Inserts a new blueprint.
    pub fn put_blueprint(&mut self, address: Address, blueprint: Blueprint) -> &mut Blueprint {
        self.updated_blueprints.insert(address);

        self.blueprints.insert(address, blueprint);
        self.blueprints.get_mut(&address).unwrap()
    }

    /// Returns an immutable reference to a component, if exists.
    pub fn get_component(&mut self, address: Address) -> Option<&Component> {
        if self.components.contains_key(&address) {
            return self.components.get(&address);
        }

        if let Some(component) = self.ledger.get_component(address) {
            self.components.insert(address, component);
            self.components.get(&address)
        } else {
            None
        }
    }
    /// Returns a mutable reference to a component, if exists.
    pub fn get_component_mut(&mut self, address: Address) -> Option<&mut Component> {
        self.updated_components.insert(address);

        if self.components.contains_key(&address) {
            return self.components.get_mut(&address);
        }

        if let Some(component) = self.ledger.get_component(address) {
            self.components.insert(address, component);
            self.components.get_mut(&address)
        } else {
            None
        }
    }

    /// Inserts a new component.
    pub fn put_component(&mut self, address: Address, component: Component) -> &mut Component {
        self.updated_components.insert(address);

        self.components.insert(address, component);
        self.components.get_mut(&address).unwrap()
    }

    /// Returns an immutable reference to a account, if exists.
    #[allow(dead_code)]
    pub fn get_account(&mut self, address: Address) -> Option<&Account> {
        if self.accounts.contains_key(&address) {
            return self.accounts.get(&address);
        }

        if let Some(account) = self.ledger.get_account(address) {
            self.accounts.insert(address, account);
            self.accounts.get(&address)
        } else {
            None
        }
    }

    /// Returns a mutable reference to a account, if exists.
    pub fn get_account_mut(&mut self, address: Address) -> Option<&mut Account> {
        self.updated_accounts.insert(address);

        if self.accounts.contains_key(&address) {
            return self.accounts.get_mut(&address);
        }

        if let Some(account) = self.ledger.get_account(address) {
            self.accounts.insert(address, account);
            self.accounts.get_mut(&address)
        } else {
            None
        }
    }

    /// Inserts a new account.
    pub fn put_account(&mut self, address: Address, account: Account) -> &mut Account {
        self.updated_accounts.insert(address);

        self.accounts.insert(address, account);
        self.accounts.get_mut(&address).unwrap()
    }

    /// Returns an immutable reference to a resource, if exists.
    pub fn get_resource(&mut self, address: Address) -> Option<&Resource> {
        if self.resources.contains_key(&address) {
            return self.resources.get(&address);
        }

        if let Some(resource) = self.ledger.get_resource(address) {
            self.resources.insert(address, resource);
            self.resources.get(&address)
        } else {
            None
        }
    }

    /// Returns a mutable reference to a resource, if exists.
    #[allow(dead_code)]
    pub fn get_resource_mut(&mut self, address: Address) -> Option<&mut Resource> {
        self.updated_resources.insert(address);

        if self.resources.contains_key(&address) {
            return self.resources.get_mut(&address);
        }

        if let Some(resource) = self.ledger.get_resource(address) {
            self.resources.insert(address, resource);
            self.resources.get_mut(&address)
        } else {
            None
        }
    }

    /// Inserts a new resource.
    pub fn put_resource(&mut self, address: Address, resource: Resource) {
        self.updated_resources.insert(address);

        self.resources.insert(address, resource);
    }

    /// Creates a new blueprint address.
    pub fn new_blueprint_address(&mut self, code: &[u8]) -> Address {
        Address::Blueprint(sha256_twice(code).lower_26_bytes())
    }

    /// Creates a new component address.
    pub fn new_component_address(&mut self) -> Address {
        let mut data = self.tx_hash.as_ref().to_vec();
        data.extend(self.component_counter.to_le_bytes());

        let hash = sha256_twice(data);
        Address::Component(hash.lower_26_bytes())
    }

    /// Creates a new resource address.
    pub fn new_resource_address(&self, owner: Address, symbol: &str) -> Address {
        let mut data: Vec<u8> = owner.into();
        data.extend(symbol.as_bytes());

        let hash = sha256_twice(data);
        Address::Resource(hash.lower_26_bytes())
    }

    /// Creates a new transient bucket id.
    pub fn new_bid(&mut self) -> BID {
        self.bucket_counter += 1;
        BID::Transient(self.bucket_counter - 1)
    }

    /// Flush changes to ledger.
    pub fn flush(&mut self) {
        let mut addresses = self.updated_blueprints.clone();
        for address in addresses {
            println!("Updating: {:?}", address);
            self.ledger
                .put_blueprint(address, self.blueprints.get(&address).unwrap().clone());
        }

        addresses = self.updated_components.clone();
        for address in addresses {
            println!("Updating: {:?}", address);
            self.ledger
                .put_component(address, self.components.get(&address).unwrap().clone());
        }

        addresses = self.updated_accounts.clone();
        for address in addresses {
            println!("Updating: {:?}", address);
            self.ledger
                .put_account(address, self.accounts.get(&address).unwrap().clone());
        }

        addresses = self.updated_resources.clone();
        for address in addresses {
            println!("Updating: {:?}", address);
            self.ledger
                .put_resource(address, self.resources.get(&address).unwrap().clone());
        }
    }
}