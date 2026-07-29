//! templates.rs
//! 
//! This module defines the "blueprints" for various geometries.
//! These templates are used by the Scene to create instanced meshes
//! for high-performance rendering.


use glam::{DVec3, Mat4 as GMat4, Vec3 as GVec3};
use crate::md_sim::{ParticleVec, BoxSpec};
use three_d::{Context, CpuMesh, Gm, InstancedMesh, Instances, Srgba, PhysicalMaterial,
    Blend, BlendEquationType, BlendMultiplierType, Cull, DepthTest,
    RenderStates, WriteMask};

///Used by all shapes except SphereTemplate which is used for particles
pub enum ObjectTemplate {
    HollowBox(BoxTemplate),
    WireBox(WireBoxTemplate),
}

impl ObjectTemplate {
    /// Returns a reference to the underlying `three-d` object for rendering
    pub fn as_object(&self) -> &(dyn three_d::Object + 'static) {
        match self {
            ObjectTemplate::HollowBox(b) => &b.mesh,
            ObjectTemplate::WireBox(w) => &w.mesh,
        }
    }
}

// -----------------------------------------------------------------------------------
// Instancing Templates
// -----------------------------------------------------------------------------------


/// Sphere is used as a base for the rendering of spherical particles
pub struct SphereTemplate {
    pub mesh: Gm<InstancedMesh, PhysicalMaterial>,
}

impl SphereTemplate {
    pub fn new(context: &Context) -> Self {
        let cpu_mesh = CpuMesh::sphere(16);
        
        let mat = create_transparent_material();

        let mesh = Gm::new(
            InstancedMesh::new(context, &Instances::default(), &cpu_mesh),
            mat
        );

        Self { mesh }
    }

    // Helper to update instance of particle
    pub fn push_transform_and_color(&self, i: usize, particles: &ParticleVec, transforms: &mut Vec<three_d::Mat4>, colors: &mut Vec<Srgba>) {
        let glam_mat = GMat4::from_translation(GVec3::new(particles.position[i].x as f32, particles.position[i].y as f32, particles.position[i].z as f32)) 
            * GMat4::from_scale(GVec3::splat(particles.radius[i] as f32));
        
        let cols = glam_mat.to_cols_array();
        transforms.push(three_d::Mat4::from_cols(
            three_d::Vector4::new(cols[0], cols[1], cols[2], cols[3]),
            three_d::Vector4::new(cols[4], cols[5], cols[6], cols[7]),
            three_d::Vector4::new(cols[8], cols[9], cols[10], cols[11]),
            three_d::Vector4::new(cols[12], cols[13], cols[14], cols[15]),
        ));
        colors.push(particles.color[i]);
    }
}


/// Wire framed box primarily used to indicate the simulation box. 
/// If you use a box dimension a negative thickness
/// preserves the outer dimension whilst a positive one preserves the inner.
pub struct WireBoxTemplate {
    pub mesh: Gm<InstancedMesh, PhysicalMaterial>,
    pub boxspec: BoxSpec,
}

impl WireBoxTemplate {
    pub fn new(context: &Context, boxspec: BoxSpec) -> Self {
        let half_size = boxspec.box_size * 0.5;
        let min = -half_size;
        let max = half_size;
        let thickness = boxspec.thickness;
        let abs_t = thickness.abs();

        let (effective_min, effective_max) = if thickness > 0.0 {
            (min - DVec3::splat(abs_t), max + DVec3::splat(abs_t))
        } else {
            (min, max)
        };

        let strut_thickness = if thickness == 0.0 { 0.0 } else { abs_t };
        let size = effective_max - effective_min;

        let c0 = DVec3::new(effective_min.x, effective_min.y, effective_min.z);
        let c1 = DVec3::new(effective_max.x, effective_min.y, effective_min.z);
        let c2 = DVec3::new(effective_min.x, effective_max.y, effective_min.z);
        let c3 = DVec3::new(effective_max.x, effective_max.y, effective_min.z);
        let c4 = DVec3::new(effective_min.x, effective_min.y, effective_max.z);
        let c5 = DVec3::new(effective_max.x, effective_min.y, effective_max.z);
        let c6 = DVec3::new(effective_min.x, effective_max.y, effective_max.z);
        let c7 = DVec3::new(effective_max.x, effective_max.y, effective_max.z);

        let edges = [
            (0.5 * (c0 + c1), DVec3::new(size.x, strut_thickness, strut_thickness)),
            (0.5 * (c1 + c3), DVec3::new(strut_thickness, size.y, strut_thickness)),
            (0.5 * (c3 + c2), DVec3::new(size.x, strut_thickness, strut_thickness)),
            (0.5 * (c2 + c0), DVec3::new(strut_thickness, size.y, strut_thickness)),
            (0.5 * (c4 + c5), DVec3::new(size.x, strut_thickness, strut_thickness)),
            (0.5 * (c5 + c7), DVec3::new(strut_thickness, size.y, strut_thickness)),
            (0.5 * (c7 + c6), DVec3::new(size.x, strut_thickness, strut_thickness)),
            (0.5 * (c6 + c4), DVec3::new(size.x, strut_thickness, strut_thickness)),
            (0.5 * (c0 + c4), DVec3::new(strut_thickness, strut_thickness, size.z)),
            (0.5 * (c1 + c5), DVec3::new(strut_thickness, strut_thickness, size.z)),
            (0.5 * (c2 + c6), DVec3::new(strut_thickness, strut_thickness, size.z)),
            (0.5 * (c3 + c7), DVec3::new(strut_thickness, strut_thickness, size.z)),
        ];

        let local_transformations: Vec<three_d::Mat4> = edges
            .iter()
            .map(|(midpoint, scale)| {
                let glam_mat = GMat4::from_translation(GVec3::new(
                    midpoint.x as f32,
                    midpoint.y as f32,
                    midpoint.z as f32,
                )) * GMat4::from_scale(GVec3::new(
                    scale.x as f32,
                    scale.y as f32,
                    scale.z as f32,
                ));

                glam_to_three_d(glam_mat)
            })
            .collect();

        let mat = create_transparent_material();
        let mesh = Gm::new(
            InstancedMesh::new(
                context,
                &Instances {
                    transformations: local_transformations,
                    ..Default::default()
                },
                &CpuMesh::cube(),
            ),
            mat,
        );

        Self { mesh, boxspec }
    }

    // Helper to update instance of particle
    pub fn push_transform_and_color(
        &mut self, // Note: This must now be mutable
        spec: &ObjectSpec,
        transforms: &mut Vec<three_d::Mat4>,
        colors: &mut Vec<three_d::Srgba>,
    ) {
        if self.boxspec == *boxspec {
            return;
        }

        let pos = boxspec.position;
        let glam_mat = GMat4::from_rotation_translation(
            glam::DQuat::from(boxspec.orientation).as_quat(),
            glam::Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32),
        );

        transforms.push(glam_to_three_d(glam_mat));
        
        colors.push(boxspec.color);
        
        // Update the stored specification so it doesn't process again unnecessarily 
        self.boxspec = *boxspec; 
    }
}

fn glam_to_three_d(mat: GMat4) -> three_d::Mat4 {
    let cols = mat.to_cols_array();
    three_d::Mat4::from_cols(
        three_d::Vector4::new(cols[0], cols[1], cols[2], cols[3]),
        three_d::Vector4::new(cols[4], cols[5], cols[6], cols[7]),
        three_d::Vector4::new(cols[8], cols[9], cols[10], cols[11]),
        three_d::Vector4::new(cols[12], cols[13], cols[14], cols[15]),
    )
}
/// This box has filled sides and a hollow center. If you use a box dimension a negative thickness
/// preserves the outer dimension whilst a positive one preserves the inner.
pub struct BoxTemplate {
    pub mesh: Gm<InstancedMesh, PhysicalMaterial>,
    pub boxspec: BoxSpec,
}

impl BoxTemplate {
    /// Creates a hollow box with filled faces from a BoxSpec.
    pub fn new(context: &Context, boxspec: BoxSpec) -> Self {
        let half_size = boxspec.box_size * 0.5;
        let min = -half_size;
        let max = half_size;
        let thickness = boxspec.thickness;
        let abs_t = thickness.abs();

        let (outer_min, outer_max, inner_min, inner_max) = if thickness == 0.0 {
            (min, max, min, max)
        } else if thickness > 0.0 {
            (
                DVec3::new(min.x - abs_t, min.y - abs_t, min.z - abs_t),
                DVec3::new(max.x + abs_t, max.y + abs_t, max.z + abs_t),
                min,
                max,
            )
        } else {
            (
                min,
                max,
                DVec3::new(min.x + abs_t, min.y + abs_t, min.z + abs_t),
                DVec3::new(max.x - abs_t, max.y - abs_t, max.z - abs_t),
            )
        };

        let walls = [
            // Left wall
            (
                0.5 * DVec3::new(outer_min.x + inner_min.x, outer_min.y + outer_max.y, outer_min.z + outer_max.z),
                DVec3::new(inner_min.x - outer_min.x, outer_max.y - outer_min.y, outer_max.z - outer_min.z),
            ),
            // Right wall
            (
                0.5 * DVec3::new(outer_max.x + inner_max.x, outer_min.y + outer_max.y, outer_min.z + outer_max.z),
                DVec3::new(outer_max.x - inner_max.x, outer_max.y - outer_min.y, outer_max.z - outer_min.z),
            ),
            // Bottom wall
            (
                0.5 * DVec3::new(outer_min.x + outer_max.x, outer_min.y + inner_min.y, outer_min.z + outer_max.z),
                DVec3::new(outer_max.x - outer_min.x, inner_min.y - outer_min.y, outer_max.z - outer_min.z),
            ),
            // Top wall
            (
                0.5 * DVec3::new(outer_min.x + outer_max.x, outer_max.y + inner_max.y, outer_min.z + outer_max.z),
                DVec3::new(outer_max.x - outer_min.x, outer_max.y - inner_max.y, outer_max.z - outer_min.z),
            ),
            // Back wall
            (
                0.5 * DVec3::new(outer_min.x + outer_max.x, outer_min.y + outer_max.y, outer_min.z + inner_min.z),
                DVec3::new(outer_max.x - outer_min.x, outer_max.y - outer_min.y, inner_min.z - outer_min.z),
            ),
            // Front wall
            (
                0.5 * DVec3::new(outer_min.x + outer_max.x, outer_min.y + outer_max.y, outer_max.z + inner_min.z),
                DVec3::new(outer_max.x - outer_min.x, outer_max.y - outer_min.y, outer_max.z - inner_min.z),
            ),
        ];

        let local_transformations: Vec<three_d::Mat4> = walls
            .iter()
            .map(|(translation, scale)| {
                let glam_mat = GMat4::from_translation(GVec3::new(
                    translation.x as f32,
                    translation.y as f32,
                    translation.z as f32,
                )) * GMat4::from_scale(GVec3::new(
                    scale.x as f32,
                    scale.y as f32,
                    scale.z as f32,
                ));

                glam_to_three_d(glam_mat)
            })
            .collect();

        let mat = create_transparent_material();
        let mesh = Gm::new(
            InstancedMesh::new(
                context,
                &Instances {
                    transformations: local_transformations,
                    ..Default::default()
                },
                &CpuMesh::cube(),
            ),
            mat,
        );

        Self { mesh, boxspec }
    }

    pub fn push_transform_and_color(
        &mut self,
        boxspec: &BoxSpec,
        transforms: &mut Vec<three_d::Mat4>,
        colors: &mut Vec<three_d::Srgba>,
    ) {
        if self.boxspec == *boxspec {
            return;
        }

        let pos = boxspec.position;
        let glam_mat = GMat4::from_rotation_translation(
            glam::DQuat::from(boxspec.orientation).as_quat(),
            glam::Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32),
        );

        transforms.push(glam_to_three_d(glam_mat));
        colors.push(boxspec.color);
        
        self.boxspec = *boxspec;
    }
}

//---------------------------------------------------------------------------------------
// Material
//--------------------------------------------------------------------------------------
fn create_transparent_material() -> PhysicalMaterial {
    let mut mat = PhysicalMaterial::default();
    mat.albedo = Srgba::WHITE;
    mat.render_states = RenderStates {
        blend: Blend::Enabled {
            source_rgb_multiplier: BlendMultiplierType::SrcAlpha,
            source_alpha_multiplier: BlendMultiplierType::One,
            destination_rgb_multiplier: BlendMultiplierType::OneMinusSrcAlpha,
            destination_alpha_multiplier: BlendMultiplierType::OneMinusSrcAlpha,
            rgb_equation: BlendEquationType::Add,
            alpha_equation: BlendEquationType::Add,
        },
        cull: Cull::Back,
        write_mask: WriteMask::COLOR, 
        depth_test: DepthTest::Always,
    };
    mat
}
