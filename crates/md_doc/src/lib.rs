#![doc = include_str!("../README.md")]

//! # Molecular Dynamics Simulation in Rust
//!
//! A complete workspace for particle simulation and visualization.
//!
//! ## Core Crates
//!
//! - **[`md_core`]** — Fundamental particle data structures
//! - **[`md_sim`]** — Simulation engine and time stepping
//! - **[`md_viz`]** — 3D visualization and rendering
//!
//! ## Quick Start
//!
//! Run the development example:
//! ```bash
//! cargo run --example develop
//! ```
//!
//! ## Architecture
//!
//! - `md_core` defines [`Particle`](md_core::particle::Particle) — the core simulation unit
//! - `md_sim` implements [`Simulation`](md_sim::simulation::Simulation) — the main engine
//! - `md_viz` defines [`Scene`] which provides visualization. All objects implement a [`Draw`](md_viz::draw_particles::Draw) trait.


