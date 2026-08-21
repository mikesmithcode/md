//! camera.rs
//!
//! This module defines the functionality of cameras for viewing the scene.
//! You have two choices: orthographic (distance doesn't matter, perfect for 2D in 3D scene) and perspective (things further away look smaller) accessed as options on an enum. You can interact with
//! the view using your mouse: zoom in and out with the wheel, rotate by holding down left button and dragging. This live camera only works on the live window not on the headless images. For these the view is set at compile time. You'd need to update the config. Changes in the live window print details to the terminal so you can use this to figure out what you want.
//! 
//! ## Cameras and lights
//! 
//! [`cameras_lights_info`]
//! 
pub mod cameras_lights_info {#![doc = include_str!("../../docs/cameras_lights.md")]}

use three_d::{Camera, Vector3};
use three_d::InnerSpace;
use three_d::*;

use winit::event::{WindowEvent, MouseButton, ElementState, MouseScrollDelta};
use crate::md_viz::SceneSettings;
use serde::{Serialize, Deserialize};

/// Configuration parameters defining camera placement, field of view, orientation, and projection mode.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MyCamera {
    pub cam_type: CameraView,
    pub fov: f32,
    #[serde(with = "vec3_serde")]
    pub rel_pos: Vec3,
    #[serde(with = "vec3_serde")]
    pub up: Vec3, 
}

impl Default for MyCamera {
    fn default() -> Self {
        Self {
            cam_type: CameraView::Perspective,
            fov: 45.0,
            rel_pos: Vec3::new(0.0, 0.25, 0.0),
            up: Vec3::new(0.0, 0.0, 1.0),
        }
    }
}

// A tiny reusable module to handle the conversion under the hood
mod vec3_serde {
    use super::Vec3;
    use serde::{Serialize, Deserialize, Serializer, Deserializer};

    pub fn serialize<S>(vec: &Vec3, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        [vec.x, vec.y, vec.z].serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec3, D::Error>
    where D: Deserializer<'de> {
        let arr = <[f32; 3]>::deserialize(deserializer)?;
        Ok(Vec3::new(arr[0], arr[1], arr[2]))
    }
}

/// Enum used to switch between different camera perspectives.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CameraView {
    Perspective,
    Orthographic,
}

/// Creates and returns a `Camera` instance based on the specified scene settings and viewport dimensions.
pub fn create_camera(viewport: Viewport, scene_settings: SceneSettings) -> Camera {
    match scene_settings.camera.cam_type {
        CameraView::Perspective => create_perspective_camera(viewport, scene_settings),
        CameraView::Orthographic => create_orthographic_camera(viewport, scene_settings),
    }
}

/// Creates a camera with a perspective projection, useful for viewing and orbiting around a 3D scene.
fn create_perspective_camera(viewport: Viewport, scene_settings: SceneSettings) -> Camera {
    let sim_box_size = scene_settings.sim_box.box_size;

    // Cast DVec3 (f64) components to f32 for rendering math
    let dim_x = sim_box_size.x as f32;
    let dim_y = sim_box_size.y as f32;
    let dim_z = sim_box_size.z as f32;
    
    // Add 10% buffer so the edges aren't cut off
    let buffered_x = 1.1 * dim_x;
    let buffered_y = 1.1 * dim_y;
    let buffered_z = 1.1 * dim_z;

    let centre = Vector3::new(dim_x * 0.5, dim_y * 0.5, dim_z * 0.5);
    
    let fov_deg = scene_settings.camera.fov;
    let fov_rad = fov_deg * std::f32::consts::PI / 180.0;
    let aspect = viewport.width as f32 / viewport.height as f32;

    // Calculate distance based on vertical (Y) and horizontal (X) extents
    let dist_y = (buffered_y * 0.5) / (fov_rad * 0.5).tan();
    
    // Adjust horizontal FOV based on aspect ratio
    let horizontal_fov_rad: f32 = 2.0 * ((fov_rad * 0.5).tan() * aspect).atan();
    let dist_x = (buffered_x * 0.5) / (horizontal_fov_rad * 0.5).tan();

    // Take the max distance and add half the depth (Z) to clear the front face
    let base_distance = dist_y.max(dist_x);
    let eye_distance = (base_distance + (buffered_z * 0.5)) * 1.1; // 10% extra padding

    let eye_pos = centre + scene_settings.camera.rel_pos;

    let up = Vector3::new(0.0, 0.0, 1.0);

    Camera::new_perspective(
        viewport,
        eye_pos,
        centre,
        up,
        degrees(fov_deg),
        0.01,                   
        eye_distance + buffered_z + 10.0, 
    )
}

/// Creates a camera with an orthographic projection.
/// 
/// This has no perspective scaling, making it ideal for viewing 2D simulations or 3D setups where apparent size does not vary with depth.
pub fn create_orthographic_camera(viewport: Viewport, scene_settings: SceneSettings) -> Camera {
    let sim_box_size = scene_settings.sim_box.box_size;

    let dim_x = sim_box_size.x as f32;
    let dim_y = sim_box_size.y as f32;
    let dim_z = sim_box_size.z as f32;

    let x_mid = dim_x * 0.5;
    let y_mid = dim_y * 0.5;
    let z_mid = dim_z * 0.5;
    
    let max_dim = dim_x.max(dim_y).max(dim_z);
        
    let rel_pos = scene_settings.camera.rel_pos;
    let up = scene_settings.camera.up;
    let centre = Vector3::new(x_mid, y_mid, z_mid);
    let position = centre + rel_pos;

    let z_near = position.y;
    let z_far = -centre.y;  
    let factor = 1.75 / (rel_pos.y);

    let camera = Camera::new_orthographic(
        viewport,
        position,
        centre,
        up,
        max_dim * factor,
        z_near,
        z_far
    );
    
    camera
}

/// Creates and returns an `OrbitControl` for camera manipulation.
pub fn create_control(camera: &Camera) -> OrbitControl {
    OrbitControl::new(camera.target(), 1.0, 1000.0)
}

/// Manages interactive mouse-based camera movement, rotation, panning, and zooming states.
pub struct CameraControl {
    pub distance: f32,
    pub zoom: f32,
    pub dragging: bool,
    pub panning: bool,
    pub last_cursor: (f32, f32),
    pub rotation_delta: (f32, f32),
    pub pan_delta: (f32, f32),
    pub update: bool,
    pub sync_needed: bool,
}

impl CameraControl {
    /// Initializes a new `CameraControl` instance based on the camera's initial position and look-at target.
    pub fn new(camera: &Camera, target: Vector3<f32>) -> Self {
        let camera_to_target = camera.position() - target;
        let distance = camera_to_target.magnitude();
        let zoom: f32 = camera.zoom_factor();
        Self { 
            distance, 
            zoom, 
            dragging: false,
            panning: false,
            last_cursor: (0.0, 0.0),
            rotation_delta: (0.0, 0.0), 
            pan_delta: (0.0, 0.0),
            update: false,
            sync_needed: false,
        }
    }

    /// Handles incoming `winit` window events to track mouse clicks, cursor dragging, and scroll wheel zooming.
    pub fn handle_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::MouseInput { state, button, .. } => {
                let is_pressed = *state == ElementState::Pressed;

                if *button == MouseButton::Left { 
                    self.dragging = is_pressed; 
                }
                if *button == MouseButton::Right { 
                    self.panning = is_pressed; 
                }

                // Every time a button is pressed down, we MUST reset the cursor baseline
                // to prevent the camera from "jumping" to a stale coordinate.
                if is_pressed {
                    self.sync_needed = true;
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = (position.x as f32, position.y as f32);

                // If a click just happened, we ignore the distance calculation
                // and simply record the current mouse position as the new starting point.
                if self.sync_needed || (self.last_cursor.0 == 0.0 && self.last_cursor.1 == 0.0) {
                    self.last_cursor = (x, y);
                    self.sync_needed = false;
                    return; 
                }

                // Only calculate and apply deltas if the user is holding a button
                if self.dragging || self.panning {
                    let dx = x - self.last_cursor.0;
                    let dy = y - self.last_cursor.1;
                    
                    // Update the baseline for the next frame
                    self.last_cursor = (x, y);

                    if self.dragging {
                        self.rotation_delta = (dx, dy);
                    } 
                    if self.panning {
                        self.pan_delta = (dx, dy);
                    }
                    
                    // Signal to the Scene that it needs to call update_camera()
                    self.update = true;
                } else {
                    // If not dragging, just keep track of where the mouse is 
                    // so we are ready for the next click.
                    self.last_cursor = (x, y);
                }
            }

            // --- MOUSE WHEEL (ZOOM) ---
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_amount = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(pos) => (pos.y as f32) * 0.1, 
                };

                // 1. Multiplicative zoom (the "Factor" method)
                // This makes zoom feel the same speed whether you are at 0.1 or 10.0
                let factor = 1.25f32; 
                if scroll_amount > 0.0 {
                    self.zoom *= factor;
                } else {
                    self.zoom /= factor;
                }

                // 2. Clamp zoom boundaries
                self.zoom = self.zoom.clamp(0.01, 100.0);
                
                // 3. Mark as updated for the renderer
                self.update = true;
            }

            _ => {}
        }
    }

    /// Updates the camera's zoom, rotation, or translation parameters based on accumulated interaction deltas.
    pub fn update_camera(&mut self, camera: &mut Camera, target: Vector3<f32>) {
        camera.set_zoom_factor(self.zoom);

        if self.dragging {
            let (dx, dy) = self.rotation_delta;
            let sensitivity = 0.005;
            
            // Use the 'target' passed from the scene so rotation stays centred
            camera.rotate_around_with_fixed_up(
                target,
                dx * sensitivity,
                dy * sensitivity,
            );

            self.rotation_delta = (0.0, 0.0);
        }

        if self.panning {
            let (dx, dy) = self.pan_delta;
            let sensitivity = 0.001 * (1.0 / self.zoom); 

            let right = camera.right_direction();
            let up = camera.up();
            
            let translation = right * (-dx * sensitivity) + up * (dy * sensitivity);
            camera.translate(translation);
            
            self.pan_delta = (0.0, 0.0);
        }
    }
}