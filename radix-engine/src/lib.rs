#![cfg_attr(not(feature = "std"), no_std)]

extern crate core;
#[cfg(not(any(feature = "std", feature = "alloc")))]
compile_error!("Either feature `std` or `alloc` must be enabled for this crate.");
#[cfg(all(feature = "std", feature = "alloc"))]
compile_error!("Feature `std` and `alloc` can't be enabled at the same time.");

#[cfg(not(any(feature = "moka", feature = "lru")))]
compile_error!("Either feature `moka` or `lru` must be enabled for this crate.");
#[cfg(all(feature = "moka", feature = "lru"))]
compile_error!("Feature `moka` and `lru` can't be enabled at the same time.");

/// Radix Engine kernel, defining state, ownership and (low-level) invocation semantics.
pub mod kernel;
/// Radix Engine system, defining packages (a.k.a. classes), components (a.k.a. objects) and invocation semantics.
pub mod system;
/// Radix Engine transaction interface.
pub mod transaction;

/// Native blueprints (to be moved to individual crates)
pub mod blueprints;

pub mod track;

/// Scrypto/SBOR types required by Radix Engine.
pub mod types;

pub mod errors;

pub mod utils;
pub mod vm;
