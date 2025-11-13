pub mod scene; // Declare the 'scene' module
pub mod templates;
pub mod objects;
pub mod video;
pub mod camera;
pub mod draw_particles;

// Re-export common types and functions for easier consumption
pub use crate::scene::*;
pub use crate::objects::*;
pub use crate::templates::*;
pub use crate::draw_particles::*;


