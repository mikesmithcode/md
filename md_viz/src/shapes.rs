use three_d::*;
use three_d::core::Mat4;

// Import the Particle and Simulation from your simulation crate
use md_core::particle::Particle;

//------------------------------------------------------------------------------
// Simulation box
//------------------------------------------------------------------------------
///Define simulation box
pub struct SimBox{
    pub on: bool, // turn simulation box on or off
    pub thickness: f32,
    pub sim_box_size: [f32; 3], // dimensions [x, y, z]
}

/// Creates and returns a `Gm<BoundingBox, PhysicalMaterial>` representing the simulation box.
pub fn create_simbox(context: &Context, sim_box: &SimBox) -> Option<Gm<BoundingBox, PhysicalMaterial>> {
    let mut cube_mesh = CpuMesh::cube();
    let sim_box_size = sim_box.sim_box_size;
    // Scale the mesh to the desired simulation box size
    let _ = cube_mesh.transform(Mat4::from_nonuniform_scale(
        sim_box_size[0] / 2.0, 
        sim_box_size[1] / 2.0, 
        sim_box_size[2] / 2.0,
    ));
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

///---------------------------------------------------------------------------------------
///Spheres
pub struct SphereTemplate {
    // We'll store the base CpuMesh and the shared material here.
    pub cpu_mesh: CpuMesh,
    pub material: PhysicalMaterial,
}

impl SphereTemplate{
    ///Create a new SphereTemplate
    /// 
    /// Templates are used to create instances which are used to render
    /// multiple spheres
    pub fn new(context: &three_d::Context) -> SphereTemplate {
        let cpu_mesh = CpuMesh::sphere(16);

        // Create a single, shared material.
        let material = PhysicalMaterial::new_transparent(
            &context,
            &CpuMaterial {
                albedo: Srgba { r: 255, g: 255, b: 255, a: 255 },
                ..Default::default()
            },
        );

        Self {
            cpu_mesh,
            material,
            }
    }

    ///draw a load of spheres. Takes in ref to vec of particles and renders them.
    pub fn draw(
        target: &mut RenderTarget,
        context: &three_d::Context,
        sphere_template: &SphereTemplate,
        particles: &[Particle],
        camera: &Camera,
        light: &DirectionalLight,
    ) {
        // Collect transformations for all particles.
        let transformations: Vec<Mat4> = particles
            .iter()
            .map(|p| {
                Mat4::from_translation(vec3(p.position.x as f32, p.position.y as f32, p.position.z as f32))
                * Mat4::from_scale(p.radius as f32)
            })
            .collect();

        //Collect colours for all particles
        let colors: Vec<Srgba> = particles
            .iter()
            .map(|p| p.color)
            .collect();

        // Create the Instances struct using the collected data.
        let instances = Instances {
            transformations,
            texture_transformations: None, // Not needed
            colors: Some(colors),
        };

        // Create the Gm<InstancedMesh, PhysicalMaterial> for this frame to render
        let instanced_mesh = Gm::new(
            InstancedMesh::new(
                &context,
                &instances,
                &sphere_template.cpu_mesh,
            ),
            sphere_template.material.clone(),
        );

        // Render the single instanced mesh using the provided target.
        target.clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0)); // Clear the screen first
        target.render(camera, &[&instanced_mesh], &[light]);
    }
}


//-----------------------------------------------------------------------------------
// Triangle
//-----------------------------------------------------------------------------------

/// Creates a triangle mesh
pub fn create_triangle(context: &Context) -> Gm<Mesh, ColorMaterial> {
    // Create a CPU-side mesh consisting of a single colored triangle
    let positions = vec![
        vec3(0.5, -0.5, 0.0),  // bottom right
        vec3(-0.5, -0.5, 0.0), // bottom left
        vec3(0.0, 0.5, 0.0),   // top
    ];
    let colors = vec![
        Srgba::RED,   // bottom right
        Srgba::GREEN, // bottom left
        Srgba::BLUE,  // top
    ];
    let cpu_mesh = CpuMesh {
        positions: Positions::F32(positions),
        colors: Some(colors),
        ..Default::default()
    };

    Gm::new(Mesh::new(&context, &cpu_mesh), ColorMaterial::default())
}
