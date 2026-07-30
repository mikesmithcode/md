//! lights.rs
//!
//! This module handles creation of lights

use three_d::*;
use three_d::Srgba;


/*-----------------------------------------------------------------------------------
Fns to create lights in the scene
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

