use three_d::{Camera, Vector3};
use three_d::InnerSpace;
use winit::event::{WindowEvent, MouseButton, ElementState, MouseScrollDelta};

pub struct CameraControl {
    pub distance: f32,
    pub zoom: f32,
}

impl CameraControl {
    pub fn new(camera: &Camera, target: Vector3<f32>) -> Self {
        let camera_to_target = camera.position() - target;
        let distance = camera_to_target.magnitude();
        let zoom: f32 = 1.0;
        Self { distance, zoom }
    }


    /// Handle a winit event
    pub fn handle_event(&mut self, event: &WindowEvent) {
        match event {
            // Left click
            WindowEvent::MouseInput { state, button, .. } => {
                if *button == MouseButton::Left && *state == ElementState::Pressed {
                    println!("Click!");
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
    pub fn update_camera(&self, camera: &mut Camera) {
        // Zoom towards the origin
        camera.set_zoom_factor(self.zoom);
    }
}
