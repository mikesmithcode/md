use three_d::{Camera, Vector3, InnerSpace};
use winit::event::{KeyboardInput, MouseButton, VirtualKeyCode, ElementState, WindowEvent};

pub struct CameraControl {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub last_mouse_pos: Option<(f64, f64)>,
    pub is_dragging: bool,
    pub speed: f32,
}

impl CameraControl {
    pub fn new() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            distance: 10.0,
            last_mouse_pos: None,
            is_dragging: false,
            speed: 0.5, // orbit zoom speed
        }
    }

    /// Handle a winit event
    pub fn handle_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::MouseInput { state, button, .. } => {
                if *button == MouseButton::Left {
                    self.is_dragging = *state == ElementState::Pressed;
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = (position.x, position.y);
                if self.is_dragging {
                    if let Some((last_x, last_y)) = self.last_mouse_pos {
                        let dx = (x - last_x) as f32;
                        let dy = (y - last_y) as f32;

                        self.yaw += dx * 0.005;
                        self.pitch += dy * 0.005;
                        self.pitch = self.pitch.clamp(-1.5, 1.5); // clamp vertical
                    }
                }
                self.last_mouse_pos = Some((x, y));
            }

            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(key) = input.virtual_keycode {
                    match (key, input.state) {
                        (VirtualKeyCode::Up, ElementState::Pressed) => self.distance -= self.speed,
                        (VirtualKeyCode::Down, ElementState::Pressed) => self.distance += self.speed,
                        _ => {}
                    }
                }
            }

            _ => {}
        }
    }

    /// Apply control state to a camera
    pub fn update_camera(&self, camera: &mut Camera) {
        // Orbit around origin
        camera.rotate_around_with_fixed_up(
            Vector3::new(0.0, 0.0, 0.0), // orbit around origin
            self.yaw,                     // horizontal rotation
            self.pitch,                   // vertical rotation
        );

        // Apply zoom based on current distance
        let camera_to_origin = camera.position() - Vector3::new(0.0, 0.0, 0.0);
        let current_distance = camera_to_origin.magnitude();

        camera.zoom(
            self.distance - current_distance, // zoom amount
            0.1,                              // min distance
            100.0,                            // max distance
        );
    }
}
