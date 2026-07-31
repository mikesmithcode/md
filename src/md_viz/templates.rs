//! templates.rs
//! 
//! This module defines the "blueprints" for various geometries.
//! These templates are used by the Scene to create instanced meshes
//! for high-performance rendering.


use glam::{DVec3, Mat4 as GMat4, Vec3 as GVec3};
use crate::md_sim::{ParticleVec, BoxSpec, ObjectSpec};
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
    pub fn get_mesh(&self) -> &(dyn three_d::Object + 'static) {
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
        let center = boxspec.position;
        let half_size = boxspec.box_size * 0.5;
        let thickness = boxspec.thickness.abs();

        // Compute effective outer bounds including thickness
        let (e_min, e_max) = if boxspec.thickness > 0.0 {
            (center - half_size - DVec3::splat(thickness), center + half_size + DVec3::splat(thickness))
        } else {
            (center - half_size, center + half_size)
        };

        let strut_t = if boxspec.thickness == 0.0 { 0.0 } else { thickness };
        let span = e_max - e_min;
        
        // Net length of the strut (excluding corner overlaps)
        let net_span = span;// - DVec3::splat(strut_t * 2.0);
        let half_t = strut_t * 0.5;

        // Define the base coordinates for the inner core corners
        let x0 = e_min.x + half_t;
        let x1 = e_max.x - half_t;
        let y0 = e_min.y + half_t;
        let y1 = e_max.y - half_t;
        let z0 = e_min.z + half_t;
        let z1 = e_max.z - half_t;

        // 3. Define the 12 edge midpoints directly at the center of each strut,
        // and scales (halved for CpuMesh::cube() which spans from -1.0 to 1.0).
        let edges = [
            // --- X-axis aligned edges (4 bottom, 4 top) ---
            // Y and Z are shifted to e_min/e_max faces offset by half_t
            (DVec3::new(center.x, y0, z0), DVec3::new(net_span.x, strut_t, strut_t) * 0.5),
            (DVec3::new(center.x, y1, z0), DVec3::new(net_span.x, strut_t, strut_t) * 0.5),
            (DVec3::new(center.x, y0, z1), DVec3::new(net_span.x, strut_t, strut_t) * 0.5),
            (DVec3::new(center.x, y1, z1), DVec3::new(net_span.x, strut_t, strut_t) * 0.5),

            // --- Y-axis aligned edges (4 vertical bottom-to-top) ---
            // X and Z are shifted to e_min/e_max faces offset by half_t
            (DVec3::new(x0, center.y, z0), DVec3::new(strut_t, net_span.y, strut_t) * 0.5),
            (DVec3::new(x1, center.y, z0), DVec3::new(strut_t, net_span.y, strut_t) * 0.5),
            (DVec3::new(x0, center.y, z1), DVec3::new(strut_t, net_span.y, strut_t) * 0.5),
            (DVec3::new(x1, center.y, z1), DVec3::new(strut_t, net_span.y, strut_t) * 0.5),

            // --- Z-axis aligned edges (4 front-to-back) ---
            // X and Y are shifted to e_min/e_max faces offset by half_t
            (DVec3::new(x0, y0, center.z), DVec3::new(strut_t, strut_t, net_span.z) * 0.5),
            (DVec3::new(x1, y0, center.z), DVec3::new(strut_t, strut_t, net_span.z) * 0.5),
            (DVec3::new(x0, y1, center.z), DVec3::new(strut_t, strut_t, net_span.z) * 0.5),
            (DVec3::new(x1, y1, center.z), DVec3::new(strut_t, strut_t, net_span.z) * 0.5),
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
        spec: &ObjectSpec
    ) {
        let boxspec = spec.get_spec();
        //If nothing has changed ignore
        if self.boxspec == boxspec {
            return;
        }

        //otherwise update position and colour.
        let pos = boxspec.position;
        let glam_mat = GMat4::from_rotation_translation(
            glam::DQuat::from(boxspec.orientation).as_quat(),
            glam::Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32),
        );

        // Update the mesh's transform directly
        self.mesh.set_transformation(glam_to_three_d(glam_mat));
        
        // If your material/color needs updating:
        self.mesh.material.albedo = boxspec.color;
        
        // Update the stored specification so it only triggers with a change
        self.boxspec = boxspec; 
    }
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
    let center = boxspec.position;
    let half_size = boxspec.box_size * 0.5;
    let thickness = boxspec.thickness;
    let abs_t = thickness.abs();

    // Compute inner and outer bounds depending on the sign of thickness,
    // mirroring the logic pattern used for the wire box struts.
    let (outer_min, outer_max, inner_min, inner_max) = if thickness == 0.0 {
        let min = center - half_size;
        let max = center + half_size;
        (min, max, min, max)
    } else if thickness > 0.0 {
        // Positive thickness: nominal size is inner cavity, walls grow outward (external)
        let inner_min = center - half_size;
        let inner_max = center + half_size;
        let outer_min = inner_min - DVec3::splat(abs_t);
        let outer_max = inner_max + DVec3::splat(abs_t);
        (outer_min, outer_max, inner_min, inner_max)
    } else {
        // Negative thickness: nominal size is outer boundary, walls shrink inward (internal)
        let outer_min = center - half_size;
        let outer_max = center + half_size;
        let inner_min = outer_min + DVec3::splat(abs_t);
        let inner_max = outer_max - DVec3::splat(abs_t);
        (outer_min, outer_max, inner_min, inner_max)
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
            let adjusted_scale = *scale * 0.5;

            let glam_mat = GMat4::from_translation(GVec3::new(
                translation.x as f32,
                translation.y as f32,
                translation.z as f32,
            )) * GMat4::from_scale(GVec3::new(
                adjusted_scale.x as f32,
                adjusted_scale.y as f32,
                adjusted_scale.z as f32,
            ));

            glam_to_three_d(glam_mat)
        })
        .collect();

    let mat = create_opaque_material();
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
        spec: &ObjectSpec
    ) {
        let boxspec = spec.get_spec();
        //If nothing has changed ignore
        if self.boxspec == boxspec {
            return;
        }

        //otherwise update position and colour.
        let pos = boxspec.position;
        let glam_mat = GMat4::from_rotation_translation(
            glam::DQuat::from(boxspec.orientation).as_quat(),
            glam::Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32),
        );

        // Update the mesh's transform directly
        self.mesh.set_transformation(glam_to_three_d(glam_mat));
        
        // If your material/color needs updating:
        self.mesh.material.albedo = boxspec.color;
        
        // Update the stored specification so it only triggers with a change
        self.boxspec = boxspec; 
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

fn create_opaque_material() -> PhysicalMaterial {
    let mut mat = PhysicalMaterial::default();
    mat.albedo = Srgba::WHITE;
    mat
}

//--------------------------------------------------------------------------
// Helpers
//---------------------------------------------------------------------------
fn glam_to_three_d(mat: GMat4) -> three_d::Mat4 {
    let cols = mat.to_cols_array();
    three_d::Mat4::from_cols(
        three_d::Vector4::new(cols[0], cols[1], cols[2], cols[3]),
        three_d::Vector4::new(cols[4], cols[5], cols[6], cols[7]),
        three_d::Vector4::new(cols[8], cols[9], cols[10], cols[11]),
        three_d::Vector4::new(cols[12], cols[13], cols[14], cols[15]),
    )
}