// src/lib.rs — crate root
#![cfg_attr(not(feature = "std"), no_std)]

pub mod constants;
pub mod math;
pub mod manifold;
pub mod ghost;
pub mod containment;
pub mod trace;
pub mod core;
pub mod pipeline;
pub mod abi;
