#![doc = include_str!("../../../README.md")]

//! # MD Sim: A Molecular Dynamics Engine
//! 
//! This crate provides the core engine for molecular dynamics simulations.
//! 
//! ## Architecture
//! The simulation revolves around the `Simulation` struct and the `update()` loop.
//! Data flows from the simulation into the `md_viz` crate for rendering.
//!
//! ### Example
//! ```rust
//! // You can even include runnable code examples here!
//! let mut sim = Simulation::new();
//! sim.update();
//! ```



pub mod simulation;
pub mod file_io;

pub use md_core::particle::*;








