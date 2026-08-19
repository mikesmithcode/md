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

use crate::md_sim::{ParticleVec, BoxSpec, RectSpec, TriSpec, ObjectSpec};

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
    pub fn new(context: &Context, particles: &ParticleVec) -> Self {
        let cpu_mesh = CpuMesh::sphere(16);

        let colour = Srgba::new(
            particles.color[0].r,
            particles.color[0].g,
            particles.color[0].b,
            particles.color[0].a
        );

        let mat = if colour.a < 255 { 
            create_transparent_material(Some(colour))
        } else {
            create_opaque_material(Some(colour))
        };

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

    pub fn push_colour_and_visibility(&self, i: usize, particles: &ParticleVec, colours: &mut Vec<Srgba>) {
        colours.push(particles.color[i]);
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
        let local_transformations = Self::construct_template(&boxspec);
        let mat = create_opaque_material(None);
        
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

    fn construct_template(boxspec: &BoxSpec) -> Vec<three_d::Mat4> {
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
        let half_t = strut_t * 0.5;

        // Define base coordinates for the inner core corners
        let x0 = e_min.x + half_t;
        let x1 = e_max.x - half_t;
        let y0 = e_min.y + half_t;
        let y1 = e_max.y - half_t;
        let z0 = e_min.z + half_t;
        let z1 = e_max.z - half_t;

        let edges = [
            // --- X-axis aligned edges (4 bottom, 4 top) ---
            (DVec3::new(center.x, y0, z0), DVec3::new(span.x, strut_t, strut_t) * 0.5),
            (DVec3::new(center.x, y1, z0), DVec3::new(span.x, strut_t, strut_t) * 0.5),
            (DVec3::new(center.x, y0, z1), DVec3::new(span.x, strut_t, strut_t) * 0.5),
            (DVec3::new(center.x, y1, z1), DVec3::new(span.x, strut_t, strut_t) * 0.5),

            // --- Y-axis aligned edges (4 vertical bottom-to-top) ---
            (DVec3::new(x0, center.y, z0), DVec3::new(strut_t, span.y, strut_t) * 0.5),
            (DVec3::new(x1, center.y, z0), DVec3::new(strut_t, span.y, strut_t) * 0.5),
            (DVec3::new(x0, center.y, z1), DVec3::new(strut_t, span.y, strut_t) * 0.5),
            (DVec3::new(x1, center.y, z1), DVec3::new(strut_t, span.y, strut_t) * 0.5),

            // --- Z-axis aligned edges (4 front-to-back) ---
            (DVec3::new(x0, y0, center.z), DVec3::new(strut_t, strut_t, span.z) * 0.5),
            (DVec3::new(x1, y0, center.z), DVec3::new(strut_t, strut_t, span.z) * 0.5),
            (DVec3::new(x0, y1, center.z), DVec3::new(strut_t, strut_t, span.z) * 0.5),
            (DVec3::new(x1, y1, center.z), DVec3::new(strut_t, strut_t, span.z) * 0.5),
        ];

        edges
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
            .collect()
    }

    // Called every frame during render/display (fast path)
    pub fn update_transform(&mut self, spec: &ObjectSpec) {
        let boxspec = spec.get_box_spec().expect("Not valid boxspec");
        
        let pos = boxspec.center;
        let glam_mat = GMat4::from_rotation_translation(
            glam::DQuat::from(boxspec.orientation).as_quat(),
            glam::Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32),
        );

        self.mesh.set_transformation(glam_to_three_d(glam_mat));
    }

    // Called during update_templates() when colors or properties change
    pub fn update_colour(&mut self, spec: &ObjectSpec) {
        let boxspec = spec.get_box_spec().expect("Not valid boxspec");
        
        if self.boxspec == boxspec {
            return;
        }

        self.mesh.material.albedo = boxspec.color;
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
        template.update_transform(&ObjectSpec::Rectangle(rectspec));
        template.update_colour(&ObjectSpec::Rectangle(rectspec));
        template
    }

    // Called every frame during render/display (fast path for positions/orientations)
    pub fn update_transform(&mut self, spec: &ObjectSpec) {
        let rectspec = match spec.get_rect_spec() {
            Some(s) => s,
            None => return,
        };

        let translation = three_d::Mat4::from_translation(three_d::Vec3::new(
            rectspec.center.x as f32, 
            rectspec.center.y as f32, 
            rectspec.center.z as f32,
        ));

        // Get the 3x3 rotation matrix from glam and extract its axes
        let m = glam::DMat3::from_quat(rectspec.orientation);
        
        // Construct three-d / cgmath compatible column vectors
        let rotation = three_d::Mat4::from_cols(
            three_d::Vec4::new(m.x_axis.x as f32, m.x_axis.y as f32, m.x_axis.z as f32, 0.0),
            three_d::Vec4::new(m.y_axis.x as f32, m.y_axis.y as f32, m.y_axis.z as f32, 0.0),
            three_d::Vec4::new(m.z_axis.x as f32, m.z_axis.y as f32, m.z_axis.z as f32, 0.0),
            three_d::Vec4::new(0.0, 0.0, 0.0, 1.0),
        );

        let scale_mat = three_d::Mat4::from_nonuniform_scale(
            rectspec.half_size.x as f32,
            rectspec.half_size.y as f32,
            0.01,
        );

        let transform = translation * rotation * scale_mat;
        self.mesh.set_transformation(transform);
    }

    // Called during update_templates() when colors or properties change
    pub fn update_colour(&mut self, spec: &ObjectSpec) {
        let rectspec = match spec.get_rect_spec() {
            Some(s) => s,
            None => return,
        };

        if self.rectspec == rectspec {
            return;
        }

        self.mesh.material.albedo = rectspec.color;
        
        self.mesh.material.render_states = three_d::RenderStates {
            write_mask: three_d::WriteMask::COLOR_AND_DEPTH,
            cull: three_d::Cull::None, 
            ..Default::default()
        };

        self.rectspec = rectspec;
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

        let mat = if trispec.color.a < 255 { 
            create_transparent_material(Some(trispec.color))
        } else {
            create_opaque_material(Some(trispec.color))
        };

        // Create the mesh with default instances
        let mesh = Gm::new(
            InstancedMesh::new(
                context,
                &Instances::default(),
                &cpu_mesh,
            ),
            mat,
        );

        // Initialize with default/empty spec data so the helper methods have a valid baseline
        let mut template = Self { 
            mesh, 
            trispec // or a default/zeroed spec if you prefer
        };
        
        template.update_transform(&ObjectSpec::Triangle(trispec));
        template.update_colour(&ObjectSpec::Triangle(trispec));

        template
    }


    pub fn update_transform(&mut self, spec: &ObjectSpec) {
        let trispec = match spec {
            ObjectSpec::Triangle(t) => t,
            _ => return,
        };

        // Always update the transform matrix so moving objects stay smooth
        let mat_transform = Self::compute_transformation(trispec);
        self.mesh.geometry.set_instances(&Instances {
            transformations: vec![mat_transform],
            ..Default::default()
        });
    }

    pub fn update_colour(&mut self, spec: &ObjectSpec) {
        let trispec = match spec {
            ObjectSpec::Triangle(t) => t,
            _ => return,
        };

        // If the spec hasn't changed, skip expensive material/render-state updates
        if &self.trispec == trispec {
            return;
        }

        self.mesh.material.albedo = trispec.color;

        // Disable back-face culling to ensure it renders from any angle
        self.mesh.material.render_states = three_d::RenderStates {
            write_mask: three_d::WriteMask::COLOR_AND_DEPTH,
            cull: three_d::Cull::None,
            ..Default::default()
        };

        self.trispec = *trispec;
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
        mat.albedo = Srgba::WHITE;
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
        mat.albedo = Srgba::WHITE;
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
