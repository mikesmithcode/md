use three_d::*;
use cgmath::{Matrix3, SquareMatrix};
use three_d::core::Mat4;

// Import the Particle and Simulation from your simulation crate
//use md_sim::{Particle, Simulation};

// For shared mutable state within the render loop
use std::rc::Rc;
use std::cell::RefCell;



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
/// Represents different camera perspectives.
#[derive(Debug, Clone, Copy)]
pub enum Perspective {
    Perspective,
    Orthographic,
    TwoDimensional,
}

pub struct CameraSettings {
    pub perspective: Perspective, // or Perspective::Orthographic, or Perspective::TwoDimensional
}

/// Creates and returns a `Camera` instance.
pub fn create_camera(viewport: Viewport, cam_settings: CameraSettings, sim_box_size: [f32;3]) -> Camera {

    match cam_settings.perspective {
        Perspective::Perspective => create_perspective_camera(viewport, sim_box_size),
        Perspective::Orthographic => create_orthographic_camera(viewport, sim_box_size),
    }

    
}

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

fn create_orthographic_camera(viewport: Viewport, sim_box_size: [f32;3]) -> Camera {
    
    let sim_box_max = sim_box_size.iter().cloned().fold(0.0, f32::max);
    let camera_height_units = sim_box_max * 1.5; // Adjust height to
    println!("Camera height units: {}", camera_height_units);
    println!("Viewport: {:?}", viewport);
    println!("Sim box size: {:?}", sim_box_size);
    
    Camera::new_orthographic(
        viewport,
        vec3(0.0, 0.0, sim_box_size[2]*4.0), // Eye position
        vec3(0.0, 0.0, 0.0), // Target position
        vec3(0.0, 1.0, 0.0), // Up direction
        camera_height_units,
        -10000.0,
        10000.0,
    )
}


    

/// Creates and returns an `OrbitControl` for camera manipulation.
pub fn create_control(camera: &Camera) -> OrbitControl {
    OrbitControl::new(camera.target(), 1.0, 1000.0) // Adjusted max_distance
}



/// Creates and returns a `DirectionalLight`.
pub fn create_light(context: &Context) -> DirectionalLight {
    DirectionalLight::new(context, 1.0, Srgba::WHITE, vec3(0.0, -1.0, -1.0))
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

/// Represents a small coordinate gizmo for orientation.
pub struct Gizmo {
    pub gizmo_camera: Camera,
    pub gizmo_axes: Axes,
    pub gizmo_size_pixels: f32,
    pub gizmo_padding_pixels: f32,
}

/// Creates and returns a `Gizmo` for the corner of the screen.
pub fn create_gizmo(context: &Context, sim_box_max: f32) -> Gizmo {
    let gizmo_size_pixels: f32 = 200.0;
    let gizmo_height_units = 20.0; // Defines the vertical extent of the orthographic view

    // Note: gizmo_camera initially gets a dummy viewport. It's updated in update_and_render_frame.
    let gizmo_camera = Camera::new_orthographic(
        Viewport { x: 0, y: 0, width: gizmo_size_pixels as u32, height: gizmo_size_pixels as u32 },
        vec3(0.0, 0.0, 5.0), // Eye: fixed distance on Z axis
        vec3(0.0, 0.0, 0.0), // Target: origin of the axes
        vec3(0.0, 1.0, 0.0), // Up: standard Y-up
        gizmo_height_units,  // height
        0.1,                 // z_near
        1000.0,               // z_far
    );

    let gizmo_axes = create_axes(context, sim_box_max);

    Gizmo {
        gizmo_camera,
        gizmo_axes,
        gizmo_size_pixels,
        gizmo_padding_pixels: 0.0,
    }
}

