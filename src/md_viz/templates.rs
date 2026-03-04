//! templates.rs
//! 
//! This module defines the "blueprints" for various geometries.
//! These templates are used by the Scene to create instanced meshes
//! for high-performance rendering.

use three_d::*;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Geometry {
    Sphere,
    Cube,
}

/// Data structure used to pass transformation and colour info
/// for a single component of a particle.
pub struct ParticleInstanceData {
    pub template: Geometry,
    pub transformation: Mat4,
    pub color: Srgba, 
}

// -----------------------------------------------------------------------------------
// Instancing Templates
// -----------------------------------------------------------------------------------

/// SphereTemplate: Blueprint for instanced spheres
pub struct SphereTemplate {
    pub cpu_mesh: CpuMesh,
    pub material: PhysicalMaterial,
}

impl SphereTemplate {
    pub fn new(context: &Context) -> Self {
        let cpu_mesh = CpuMesh::sphere(16);
        let material = PhysicalMaterial::new_transparent(
            context,
            &CpuMaterial {
                albedo: Srgba::WHITE,
                ..Default::default()
            },
        );

        Self {
            cpu_mesh,
            material,
        }
    }
}
