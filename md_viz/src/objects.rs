use three_d::*;
use three_d::core::Mat4;

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
        vec3(0.0, -1.0, -0.5),
    )
}

pub fn create_ambient_light(context: &Context)->AmbientLight{
    AmbientLight::new(context, 0.1, Srgba::WHITE)
}


/// Creates and returns `Axes` for visualization.
pub fn create_axes(context: &Context, sim_box_max: f32) -> Axes {

    let mut axes = Axes::new(context, 0.1, 1.0); // size, length
    let axes_offset = sim_box_max / 2.0;
        let padding = 0.5; // Small offset from the edge

        axes.set_transformation(
            Mat4::from_translation(vec3(
                axes_offset - padding,  // X position
                axes_offset - padding,  // Y position
                axes_offset - padding,  // Z position
            ))
        );
    axes
}

//------------------------------------------------------------------------------
// Simulation box
//------------------------------------------------------------------------------
///Define simulation box
#[derive(Debug, Clone, Copy)]
pub struct SimBox{
    pub on: bool, // turn simulation box on or off
    pub thickness: f32,
    pub sim_box_size: [f32; 3], // dimensions [x, y, z]
}

/// Creates and returns a `Gm<BoundingBox, PhysicalMaterial>` representing the simulation box.
pub fn create_simbox(context: &Context, sim_box: SimBox) -> Option<Gm<BoundingBox, PhysicalMaterial>> {
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


