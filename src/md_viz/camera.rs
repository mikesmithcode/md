//! camera.rs
//!
//! This module defines the functionality of cameras for viewing the scene.
//! You have two choices: orthographic (distance doesn't matter, perfect for 2D in 3D scene) and perspective (things further away look smaller) accessed as options on an enum. You can interact with
//! the view using your mouse: zoom in and out with the wheel, rotate by holding down left button and dragging. This live camera only works on the live window not on the headless images. For these the view is set at compile time. You'd need to update the config. Changes in the live window print details to the terminal so you can use this to figure out what you want.
//! 



use three_d::{Camera, Vector3};
use three_d::InnerSpace;
use three_d::*;

use winit::event::{WindowEvent, MouseButton, ElementState, MouseScrollDelta};
use crate::md_viz::scene::SceneSetup;



/// Enum used to switch between different camera perspectives.
#[derive(Debug, Clone, Copy)]
pub enum CameraView {
    Perspective,
    Orthographic,
}


/// Creates and returns a `Camera` instance.
pub fn create_camera(viewport: Viewport, scene_settings: SceneSetup) -> Camera {

    match scene_settings.camera {
        CameraView::Perspective => create_perspective_camera(viewport, scene_settings.sim_box_setup.sim_box_size),
        CameraView::Orthographic => create_orthographic_camera(viewport, scene_settings.sim_box_setup.sim_box_size),
    }

    
}

///Create a camera that has perspective. 
/// 
/// This is useful for viewing and rotating around a 3D scene
/// The default here is that +ve x is to the right, +ve y is into the page, +ve z is upwards on the page.
fn create_perspective_camera(viewport: Viewport, sim_box_size: [f32; 3]) -> Camera {
    let [dim_x, dim_y, dim_z] = sim_box_size;
    
    // 1. Add 10% buffer so the edges of the particles aren't cut off
    let buffered_x = 1.1*dim_x;
    let buffered_z = 1.1*dim_z;
    let buffered_y = 1.1*dim_y;

    let centre = vec3(dim_x * 0.5, dim_y * 0.5, dim_z * 0.5);
    
    let fov_deg = 45.0;
    let fov_rad = fov_deg * std::f32::consts::PI / 180.0;
    let aspect = viewport.width as f32 / viewport.height as f32;

    // 2. Calculate distance for Height (Z) and Width (X)
    let dist_z = (buffered_z * 0.5) / (fov_rad * 0.5).tan();
    
    // Adjust horizontal FOV based on aspect ratio
    let horizontal_fov_rad = 2.0 * ((fov_rad * 0.5).tan() * aspect).atan();
    let dist_x = (buffered_x * 0.5) / (horizontal_fov_rad * 0.5).tan();

    // 3. Take the max distance and add half the depth (Y) 
    // This ensures the camera is back far enough to see the FRONT face
    let base_distance = dist_z.max(dist_x);
    let eye_distance = (base_distance + (buffered_y * 0.5)) * 1.1; // 10% extra padding

    let eye_pos = centre + vec3(0.0, -eye_distance, 0.0);

    Camera::new_perspective(
        viewport,
        eye_pos,
        centre,
        vec3(0.0, 0.0, 1.0), 
        degrees(fov_deg),
        0.01,                
        eye_distance + buffered_y + 10.0, 
    )
}

///Create a camera with orthographic view point
/// 
/// This has no perspective. Can be useful if you want to view a 2D simulation or
/// 3D with no changes in apparent size with depth.
fn create_orthographic_camera(viewport: Viewport, sim_box_size: [f32; 3]) -> Camera {
    let x_mid = sim_box_size[0] * 0.5;
    let y_mid = sim_box_size[1] * 0.5;
    let z_mid = sim_box_size[2] * 0.5;
    let centre = vec3(x_mid, y_mid, z_mid);
    
    // Find the largest dimension to set the initial zoom level
    let max_dim = sim_box_size[0].max(sim_box_size[1]).max(sim_box_size[2]);
    
    // Initial height: 1.5x the largest dimension ensures the box fits
    let camera_height_units = max_dim * 1.5; 
    
    Camera::new_orthographic(
        viewport,
        // Place the eye directly in front of the centre along the Z-axis
        vec3(x_mid, y_mid, max_dim * 2.5), 
        centre,              // Look at the centre of the box
        vec3(0.0, 0.0, 1.0), // Up direction
        camera_height_units,
        -max_dim * 10.0,     // Near plane
        max_dim * 10.0,      // Far plane
    )
}
  

/// Creates and returns an `OrbitControl` for camera manipulation.
pub fn create_control(camera: &Camera) -> OrbitControl {
    OrbitControl::new(camera.target(), 1.0, 1000.0) // Adjusted max_distance
}

pub struct CameraControl {
    pub distance: f32,
    pub zoom: f32,
    pub dragging: bool,
    pub panning: bool,
    pub last_cursor: (f32, f32),
    pub rotation_delta: (f32,f32),
    pub pan_delta: (f32, f32),
    pub update: bool,
    pub sync_needed: bool,
}

impl CameraControl {
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
            pan_delta: (0.0,0.0),
            update: false,
            sync_needed: false,
        }
    }


    /// Handle a winit event
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

            // 2. Clamp to sensible British proportions
            self.zoom = self.zoom.clamp(0.01, 100.0);
            
            // 3. Mark as updated for the renderer
            self.update = true;
        }

        _ => {}
    }
}

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
            let sensitivity = 0.001 * (1.0/self.zoom); 

            let right = camera.right_direction();
            let up = camera.up();
            
            let translation = right * (-dx * sensitivity) + up * (dy * sensitivity);
            camera.translate(translation);
            
            self.pan_delta = (0.0, 0.0);
        }
    }
}
