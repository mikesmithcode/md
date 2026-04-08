//! objects.rs
//!
//! This module handles creation of all things in a scene other than a camera: Lights, axes, simulation box, 
//! 

use three_d::*;
use three_d::core::Mat4;

use three_d::Srgba;
use serde::{Serialize, Deserialize};

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


//------------------------------------------------------------------------------
// Simulation box
//------------------------------------------------------------------------------
///Define simulation box
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct SimBox{
    pub on: bool, // turn simulation box on or off
    pub thickness: f32,
    #[serde(skip)]
    pub sim_box_size: [f32; 3], // dimensions [x, y, z]
}

impl Default for SimBox{
    fn default() -> Self {
        Self{ on: true,
                thickness: 0.1,
                sim_box_size: [10.0, 0.1, 10.0],
            }
    }
}

/// Creates and returns a `Gm<BoundingBox, PhysicalMaterial>` representing the simulation box.
pub fn create_simbox(context: &Context, sim_box: SimBox) -> Option<Gm<BoundingBox, PhysicalMaterial>> {
    let mut cube_mesh = CpuMesh::cube();
    let sim_box_size = sim_box.sim_box_size;
    // Scale the mesh to the desired simulation box size
    let transformation = Mat4::from_translation(vec3(sim_box_size[0]/2.0, sim_box_size[1]/2.0, sim_box_size[2]/2.0)) 
                       * Mat4::from_nonuniform_scale(sim_box_size[0]/2.0, sim_box_size[1]/2.0, sim_box_size[2]/2.0);
    let _ = cube_mesh.transform(transformation);
    let thickness:f32 = sim_box.thickness;

    if sim_box.on{
        Some(Gm::new(
            BoundingBox::new_with_thickness(context, cube_mesh.compute_aabb(),thickness), // Create BoundingBox from the scaled mesh
            PhysicalMaterial::new_transparent(
                &context,
                &CpuMaterial {
                    albedo: Srgba {
                        r: 200,
                        g: 200,
                        b: 200,
                        a: 200,
                    },
                    ..Default::default()
                },
            ),
        ))}
        else{
            None
        }
}


