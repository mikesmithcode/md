use three_d::{Camera, Vector3};
use three_d::InnerSpace;
use winit::event::{WindowEvent, MouseButton, ElementState, MouseScrollDelta};

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
