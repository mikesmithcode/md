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
use crate::scene::SceneSetup;



/// Enum used to switch between different camera perspectives.
#[derive(Debug, Clone, Copy)]
pub enum Perspective {
    Perspective,
    Orthographic,
}

#[derive(Debug, Clone, Copy)]
pub struct CameraSettings {
    pub perspective: Perspective, // or Perspective::Orthographic
    pub window_dt: f64,
    pub headless_dt: f64,
}

/// Creates and returns a `Camera` instance.
pub fn create_camera(viewport: Viewport, scene_settings: SceneSetup) -> Camera {

    match scene_settings.camera.perspective {
        Perspective::Perspective => create_perspective_camera(viewport, scene_settings.sim_box_setup.sim_box_size),
        Perspective::Orthographic => create_orthographic_camera(viewport, scene_settings.sim_box_setup.sim_box_size),
    }

    
}

///Create a camera that has perspective. 
/// 
/// This is useful for viewing and rotating around a 3D scene
fn create_perspective_camera(viewport: Viewport, sim_box_size: [f32; 3]) -> Camera {
    
    Camera::new_perspective(
        viewport, 
        vec3(0.0, 0.0, sim_box_size[2]*5.0), // Camera position adjusted for a larger box
        vec3(0.0, 0.0, 0.0), // Look at the center of the simulation box
        vec3(0.0, 1.0, 0.0), // Up direction
        degrees(45.0),
        0.1,
        100000.0,
    )
}

///Create a camera with orthographic view point
/// 
/// This has no perspective. Can be useful if you want to view a 2D simulation or
/// 3D with no changes in apparent size with depth.
fn create_orthographic_camera(viewport: Viewport, sim_box_size: [f32;3]) -> Camera {
    
    let sim_box_max = sim_box_size.iter().cloned().fold(0.0, f32::max);
    let camera_height_units = 2.*sim_box_max; // Adjust height to
    
    Camera::new_orthographic(
        viewport,
        vec3(0.0, 0.0, sim_box_size[2]*0.25), // Eye position
        vec3(0.0, 0.0, 0.0), // Target position
        vec3(0.0, 1.0, 0.0), // Up direction
        camera_height_units,
        -1000.0,
        1000.0,
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
    pub last_cursor: (f32, f32),
    pub rotation_delta: (f32,f32),
}

impl CameraControl {
    pub fn new(camera: &Camera, target: Vector3<f32>) -> Self {
        let camera_to_target = camera.position() - target;
        let distance = camera_to_target.magnitude();
        let zoom: f32 = 1.0;
        Self { 
            distance, 
            zoom, 
            dragging: false,
            last_cursor: (0.0, 0.0),
            rotation_delta: (0.0, 0.0), 
        }
    }


    /// Handle a winit event
    pub fn handle_event(&mut self, event: &WindowEvent) {
        match event {
            // Left click
            WindowEvent::MouseInput { state, button, .. } => {
                if *button == MouseButton::Left {
                    self.dragging = *state == ElementState::Pressed;
                }
            }   

            // Track cursor movement
            WindowEvent::CursorMoved { position, .. } => {
                if self.dragging {
                    let (x, y) = (position.x as f32, position.y as f32);
                    let dx = x - self.last_cursor.0;
                    let dy = y - self.last_cursor.1;

                    // Apply rotation to camera in update_camera
                    self.last_cursor = (x, y);

                    // Store the delta for update step
                    self.rotation_delta = (dx, dy);
                } else {
                    self.last_cursor = (position.x as f32, position.y as f32);
                }
            }

            // Mouse wheel for zoom
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_amount = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                };
                
                //Adjust zoom
                self.zoom += scroll_amount *0.025;
                self.zoom = self.zoom.clamp(0.1, 3.0);
                println!("{}",self.zoom);
                
            }

            _ => {}
        }
    }

    /// Apply zoom to a three_d Camera
    pub fn update_camera(&mut self, camera: &mut Camera) {
        // Zoom towards the origin
        camera.set_zoom_factor(self.zoom);

        // Apply rotation if dragging
        if self.dragging {
            let (dx, dy) = self.rotation_delta;
            let sensitivity = 0.005;
            camera.rotate_around_with_fixed_up(
                Vector3::new(0.0,0.0,0.0),
                -dx * sensitivity,
                -dy * sensitivity,
            );

            // reset delta after applying
            self.rotation_delta = (0.0, 0.0);
        }
    }
}
