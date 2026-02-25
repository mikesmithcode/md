//! ---
//!
//! ## Graphics Implementation
//! 
//! This section details the internal rendering logic of the `md_viz` crate.
//!
//! ### How are things drawn?
//! 
//! 1. An **immutable reference** to a `Vec<Particle>` is passed to `scene.display()` or `scene.save_img()`.
//!    Internally, this is handed off to `self.render_particles()`.
//! 
//! 2. Each **Particle struct** has the `Draw` trait implemented.
//!    - The `.draw()` method takes a **mutable reference** to the objects vector in the `Scene`.
//!    - It uses primitive templates (e.g., spheres) to add geometry to that vector.
//! 
//! 3. **Environmental objects** like lights and cameras are also managed within the `Scene` objects vector.
//! 
//! 4. The final rendering is handled by the **three_d** crate.



pub mod scene; // Declare the 'scene' module
pub mod templates;
pub mod objects;
pub mod video;
pub mod camera;

// Re-export common types and functions for easier consumption
pub use crate::scene::*;
pub use crate::objects::*;
pub use crate::templates::*;


