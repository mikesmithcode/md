//! templates.rs
//! 
//! This module defines the "blueprints" for various geometries.
//! These templates are used by the Scene to create instanced meshes
//! for high-performance rendering.


use glam::{DVec3, Mat4 as GMat4, Vec3 as GVec3};
use three_d::{Context, CpuMesh,Mesh, Gm, InstancedMesh, Instances, Srgba, PhysicalMaterial,
    Blend, BlendEquationType, BlendMultiplierType, Cull, DepthTest,
    RenderStates, WriteMask};
use three_d::InnerSpace;

use crate::md_sim::{ParticleVec, BoxSpec, RectSpec, TriSpec, ObjectSpec, Visibility};

///Used by all shapes except SphereTemplate which is used for particles
pub enum ObjectTemplate {
    WireBox(WireBoxTemplate),
    Rectangle(RectTemplate),
    Triangle(TriTemplate),
}

impl ObjectTemplate {
    /// Returns a reference to the underlying `three-d` object for rendering
    pub fn get_mesh(&self) -> &(dyn three_d::Object + 'static) {
        match self {
            ObjectTemplate::WireBox(w) => &w.mesh,
            ObjectTemplate::Rectangle(r) => &r.mesh,
            ObjectTemplate::Triangle(t) => &t.mesh,
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

        let mat = create_transparent_material(None);

        let mesh = Gm::new(
            InstancedMesh::new(context, &Instances::default(), &cpu_mesh),
            mat
        );

        Self { mesh }
    }

    // Helper to update instance of particle
    pub fn push_transform(&self, i: usize, particles: &ParticleVec, transforms: &mut Vec<three_d::Mat4>) {
        let glam_mat = GMat4::from_translation(GVec3::new(particles.position[i].x as f32, particles.position[i].y as f32, particles.position[i].z as f32)) 
            * GMat4::from_scale(GVec3::splat(particles.radius[i] as f32));
        
        let cols = glam_mat.to_cols_array();
        transforms.push(three_d::Mat4::from_cols(
            three_d::Vector4::new(cols[0], cols[1], cols[2], cols[3]),
            three_d::Vector4::new(cols[4], cols[5], cols[6], cols[7]),
            three_d::Vector4::new(cols[8], cols[9], cols[10], cols[11]),
            three_d::Vector4::new(cols[12], cols[13], cols[14], cols[15]),
        ));
    }

    pub fn push_color_and_visibility(&self, i: usize, particles: &ParticleVec, transforms: &mut Vec<three_d::Mat4>, colors: &mut Vec<Srgba>) {
        
        colors.push(particles.color[i]);
    }
}


/// ------------------------------------------------------------------------------------
/// Wire framed box primarily used to indicate the simulation box. 
/// If you use a box dimension a negative thickness
/// preserves the outer dimension whilst a positive one preserves the inner.
/// -----------------------------------------------------------------------------------
pub struct WireBoxTemplate {
    pub mesh: Gm<InstancedMesh, PhysicalMaterial>,
    pub boxspec: BoxSpec,
}

impl WireBoxTemplate {
    pub fn new(context: &Context, boxspec: BoxSpec) -> Self {
        let center = boxspec.center;
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

        let mat = create_transparent_material(None);
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
        let boxspec = spec.get_box_spec().expect("Not valid boxspec");
        //If nothing has changed ignore
        if self.boxspec == boxspec {
            return;
        }

        //otherwise update position and colour.
        let pos = boxspec.center;
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



/// ---------------------------------------------------------------------------------
/// Flat rectangular plane template used for rendering boundaries or walls.
/// ----------------------------------------------------------------------------------
pub struct RectTemplate {
    pub mesh: Gm<Mesh, PhysicalMaterial>,
    pub rectspec: RectSpec,
}

impl RectTemplate {
    pub fn new(context: &Context, rectspec: RectSpec) -> Self {
        let mut mat = create_opaque_material(None);
        mat.albedo = rectspec.color;

        let mesh = Gm::new(
            Mesh::new(context, &CpuMesh::square()),
            mat,
        );

        let mut template = Self { mesh, rectspec: rectspec.clone() };
        template.update_transform_and_color(&rectspec);
        template
    }

    pub fn push_transform_and_color(&mut self, spec: &ObjectSpec) {
        let rectspec = match spec.get_rect_spec() {
            Some(s) => s,
            None => return,
        };

        if self.rectspec == rectspec {
            return;
        }

        self.update_transform_and_color(&rectspec);
        self.rectspec = rectspec;
    }

    fn update_transform_and_color(&mut self, rectspec: &RectSpec) {
        println!("center {:?}", rectspec.center);
        
        let translation = three_d::Mat4::from_translation(three_d::Vec3::new(
            rectspec.center.x as f32, 
            rectspec.center.y as f32, 
            rectspec.center.z as f32,
        ));

        let tangent = three_d::Vec3::new(
            rectspec.tangent.x as f32,
            rectspec.tangent.y as f32,
            rectspec.tangent.z as f32,
        ).normalize();

        let normal = three_d::Vec3::new(
            rectspec.normal.x as f32,
            rectspec.normal.y as f32,
            rectspec.normal.z as f32,
        ).normalize();

        let bitangent = normal.cross(tangent).normalize();

        // Construct the rotation matrix from the orientation frame
        let rotation = three_d::Mat4::from_cols(
            tangent.extend(0.0),
            bitangent.extend(0.0),
            normal.extend(0.0),
            three_d::Vec4::unit_w(),
        );

        let scale_mat = three_d::Mat4::from_nonuniform_scale(
            rectspec.half_size.x as f32,
            rectspec.half_size.y as f32,
            0.01,
        );

        // Include rotation between translation and scaling
        let transform = translation * rotation * scale_mat;

        self.mesh.set_transformation(transform);
        self.mesh.material.albedo = rectspec.color;
        
        // Disable back-face culling to ensure it renders from any angle
        self.mesh.material.render_states = three_d::RenderStates {
            write_mask: three_d::WriteMask::COLOR_AND_DEPTH,
            cull: three_d::Cull::None, 
            ..Default::default()
        };
    }
}
//self.mesh.set_transformation(three_d::Mat4::from_scale(0.1)); 

/// ---------------------------------------------------------------------------------
/// Custom triangular mesh template used for rendering CAD shapes or particle walls.
/// ----------------------------------------------------------------------------------
pub struct TriTemplate {
    pub mesh: Gm<InstancedMesh, PhysicalMaterial>,
    pub trispec: TriSpec,
}

impl TriTemplate {
    pub fn new(context: &Context, trispec: TriSpec) -> Self {
        let [v0, v1, v2] = trispec.local_triangles;

        let positions = vec![
            three_d::Vec3::new(v0.x as f32, v0.y as f32, v0.z as f32),
            three_d::Vec3::new(v1.x as f32, v1.y as f32, v1.z as f32),
            three_d::Vec3::new(v2.x as f32, v2.y as f32, v2.z as f32),
        ];

        let indices = three_d::Indices::U32(vec![0, 1, 2]);

        let mut cpu_mesh = CpuMesh {
            positions: three_d::Positions::F32(positions),
            indices,
            ..Default::default()
        };
        cpu_mesh.compute_normals();

        let mut mat = create_opaque_material(None);
        mat.albedo = trispec.color;

        let mat_transform = Self::compute_transformation(&trispec);

        let mesh = Gm::new(
            InstancedMesh::new(
                context,
                &Instances {
                    transformations: vec![mat_transform],
                    ..Default::default()
                },
                &cpu_mesh,
            ),
            mat,
        );

        let mut template = Self { mesh, trispec: trispec.clone() };
        template.update_transform_and_color(&trispec);
        template
    }

    pub fn push_transform_and_color(&mut self, spec: &ObjectSpec) {
        let trispec = match spec {
            ObjectSpec::Triangle(t) => t,
            _ => return,
        };

        if &self.trispec == trispec {
            return;
        }

        self.update_transform_and_color(trispec);
        self.trispec = *trispec;
    }

    fn update_transform_and_color(&mut self, trispec: &TriSpec) {
        let mat_transform = Self::compute_transformation(trispec);

        self.mesh.geometry.set_instances(&Instances {
            transformations: vec![mat_transform],
            ..Default::default()
        });
        self.mesh.material.albedo = trispec.color;

        // Disable back-face culling to ensure it renders from any angle
        self.mesh.material.render_states = three_d::RenderStates {
            write_mask: three_d::WriteMask::COLOR_AND_DEPTH,
            cull: three_d::Cull::None,
            ..Default::default()
        };
    }

    fn compute_transformation(trispec: &TriSpec) -> three_d::Mat4 {
        let translation = three_d::Mat4::from_translation(three_d::Vec3::new(
            trispec.center.x as f32,
            trispec.center.y as f32,
            trispec.center.z as f32,
        ));

        let normal = three_d::Vec3::new(
            trispec.normal.x as f32,
            trispec.normal.y as f32,
            trispec.normal.z as f32,
        ).normalize();

        let tangent = three_d::Vec3::new(
            trispec.tangent.x as f32,
            trispec.tangent.y as f32,
            trispec.tangent.z as f32,
        ).normalize();

        let bitangent = normal.cross(tangent).normalize();

        // Construct the rotation matrix from the orientation frame matching RectTemplate
        let rotation = three_d::Mat4::from_cols(
            tangent.extend(0.0),
            bitangent.extend(0.0),
            normal.extend(0.0),
            three_d::Vec4::unit_w(),
        );

        // Since the local triangle vertices are already stored at true scale relative to center,
        // no additional scaling matrix is required here (identity scale).
        translation * rotation
    }
}


//---------------------------------------------------------------------------------------
// Material
//--------------------------------------------------------------------------------------



fn create_transparent_material(colour: Option<Srgba>) -> PhysicalMaterial {
    let mut mat = PhysicalMaterial::default();
    if let Some(colour)=colour{
        mat.albedo = colour;
    }else{
        mat.albedo = Srgba::RED;
    }
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

fn create_opaque_material(colour: Option<Srgba>) -> PhysicalMaterial {
    let mut mat = PhysicalMaterial::default();
    if let Some(colour)=colour{
        mat.albedo = colour;
    }else{
        mat.albedo = Srgba::RED;
    }
    
    // Disable backface culling so surfaces render from both sides
    mat.render_states = three_d::RenderStates {
        cull: three_d::Cull::None,
        ..Default::default()
    };
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
