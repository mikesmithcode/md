pub mod scene; // Declare the 'scene' module
pub mod shapes;
pub mod objects;
pub mod video;
pub mod camera;
pub mod primitives;

// Re-export common types and functions for easier consumption
pub use crate::scene::*;
pub use crate::objects::*;
pub use crate::shapes::*;
pub use crate::primitives::*;


