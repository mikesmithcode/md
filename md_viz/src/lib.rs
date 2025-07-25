pub mod scene; // Declare the 'scene' module
pub mod shapes;
pub mod objects;

// Re-export common types and functions for easier consumption
// You might want to be more selective about what you re-export.
pub use scene::{Scene, SceneSetup};
pub use objects::{create_window, create_camera, create_control, create_light, create_axes, Gizmo, create_gizmo, Perspective, CameraSettings};
pub use shapes::{create_simbox, create_sphere};

// You might also consider creating an `errors.rs` or `types.rs` for other shared items.

