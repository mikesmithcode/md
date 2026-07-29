//! objects.rs
//!
//! This module handles creation of all things in a scene other than a camera: Lights, axes, simulation box, 
//! It is purely related to graphics

use three_d::*;
use three_d::Srgba;


/*-----------------------------------------------------------------------------------
Fns to create objects
-------------------------------------------------------------------------------------*/

/// Creates and returns a `DirectionalLight`.
/// 
/// If your contect is a HeadlessContect you need to dereference value
/// as you send it. ie *context.
pub fn create_directional_light(context: &Context)->DirectionalLight{
    DirectionalLight::new(
        context,
        1.0,
        Srgba::WHITE,
        vec3(0.0, 1.0, -0.5),
    )
}

pub fn create_ambient_light(context: &Context)->AmbientLight{
    AmbientLight::new(context, 0.1, Srgba::WHITE)
}

