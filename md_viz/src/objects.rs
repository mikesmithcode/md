use three_d::*;
use three_d::core::Mat4;

use three_d::Srgba;



use crate::scene::SceneSetup;

/*-----------------------------------------------------------------------------------
Fns to create objects
-------------------------------------------------------------------------------------*/

/// Creates and returns a `Window` instance with specified settings.
pub fn create_window(window_size:(u32,u32)) -> Window {
    Window::new(WindowSettings {
        title: "MD Visualization".to_string(), // More descriptive title
        max_size: Some(window_size),
        ..Default::default()
    })
    .unwrap()
}



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
    println!("Camera height units: {}", camera_height_units);
    println!("Viewport: {:?}", viewport);
    println!("Sim box size: {:?}", sim_box_size);
    
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


/// Creates and returns a `DirectionalLight`.
/// 
/// If your contect is a HeadlessContect you need to dereference value
/// as you send it. ie *context.
pub fn create_directional_light(context: &Context)->DirectionalLight{
    DirectionalLight::new(
        context,
        1.0,
        Srgba::WHITE,
        vec3(0.0, -1.0, -0.5),
    )
}

pub fn create_ambient_light(context: &Context)->AmbientLight{
    AmbientLight::new(context, 0.1, Srgba::WHITE)
}


/// Creates and returns `Axes` for visualization.
pub fn create_axes(context: &Context, sim_box_max: f32) -> Axes {

    let mut axes = Axes::new(context, 0.1, 1.0); // size, length
    let axes_offset = sim_box_max / 2.0;
        let padding = 0.5; // Small offset from the edge

        axes.set_transformation(
            Mat4::from_translation(vec3(
                axes_offset - padding,  // X position
                axes_offset - padding,  // Y position
                axes_offset - padding,  // Z position
            ))
        );
    axes
}



