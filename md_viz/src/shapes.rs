use three_d::*;
use cgmath::{Matrix3, SquareMatrix};
use three_d::core::Mat4;

// Import the Particle and Simulation from your simulation crate
use md_sim::{Particle, Simulation};

// For shared mutable state within the render loop
use std::rc::Rc;
use std::cell::RefCell;



pub struct SimBox{
    pub on: bool, // turn simulation box on or off
    pub sim_box_size: [f64; 3], // dimensions [x, y, z]
}

impl SimBox{
    pub fn sim_box_size_f32(&self)->[f32;3]{
        [self.sim_box_size[0] as f32, self.sim_box_size[1] as f32, self.sim_box_size[2] as f32]
    }
      
}

/// Creates and returns a `Gm<BoundingBox, PhysicalMaterial>` representing the simulation box.
pub fn create_simbox(context: &Context, sim_box: SimBox) -> Option<Gm<BoundingBox, PhysicalMaterial>> {
    let mut cube_mesh = CpuMesh::cube();
    let sim_box_size = sim_box.sim_box_size_f32();
    // Scale the mesh to the desired simulation box size
    cube_mesh.transform(Mat4::from_nonuniform_scale(sim_box_size[0], sim_box_size[1], sim_box_size[2]));
    let thickness:f32 = 2.1;

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




/// Creates and returns a `Gm<Mesh, PhysicalMaterial>` representing a particle.
pub fn create_sphere(context: &three_d::Context, particle: &Particle) -> Gm<Mesh, PhysicalMaterial> {
    let mut sphere = Gm::new(
        Mesh::new(&context, &CpuMesh::sphere(16)),
        PhysicalMaterial::new_transparent(
            &context,
            &CpuMaterial {
                albedo: Srgba {
                    r: particle.color.r,
                    g: particle.color.g,
                    b: particle.color.b,
                    a: particle.color.a,
                },
                ..Default::default()
            },
        ),
    );
    // Initial transformation will be updated per frame
    sphere.set_transformation(
        Mat4::from_translation(
            vec3(
                particle.position.x as f32,
                particle.position.y as f32,
                particle.position.z as f32,
            )
        ) * Mat4::from_scale(particle.radius as f32),
    );
    sphere
}

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
