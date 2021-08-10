#[cfg(feature = "alloc")]
pub use alloc::vec::Vec;
#[cfg(feature = "alloc")]
pub use alloc::collections::BTreeSet;
#[cfg(feature = "alloc")]
pub use alloc::collections::BTreeMap;
#[cfg(feature = "alloc")]
pub use hashbrown::HashSet;
#[cfg(feature = "alloc")]
pub use hashbrown::HashMap;

#[cfg(not(feature = "alloc"))]
pub use std::vec::Vec;
#[cfg(not(feature = "alloc"))]
pub use std::collections::BTreeSet;
#[cfg(not(feature = "alloc"))]
pub use std::collections::BTreeMap;
#[cfg(not(feature = "alloc"))]
pub use std::collections::HashSet;
#[cfg(not(feature = "alloc"))]
pub use std::collections::HashMap;