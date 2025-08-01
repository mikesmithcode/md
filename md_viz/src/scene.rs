use three_d::*;

// For shared mutable state within the render loop
use crate::shapes::SphereTemplate;
pub use crate::shapes::{create_simbox, create_sphere_template, SimBox};
pub use crate::objects::{create_window, create_camera, create_control, create_light, create_axes, CameraSettings};

use md_sim::{Particle, Simulation};


/*-----------------------------------------------------------------------------------
Create structs
-------------------------------------------------------------------------------------*/
///## SceneSetup:
/// top-level struct that defines all the visualization options such as window_size
/// It uses various lower level structs to define certain features.
/// If you want to view output from your simulation this will need to be defined in your
/// example script.
pub struct SceneSetup {
    pub camera: CameraSettings,
    pub window_size: (u32, u32),
    pub sim_box: SimBox,
}


/// The main structure encapsulating the 3D scene elements.
pub struct Scene {
    pub context: Context,
    pub camera: Camera,
    pub control: OrbitControl,
    pub simbox: Option<Gm<BoundingBox, PhysicalMaterial>>,
    pub light: DirectionalLight,
    pub sphere_template: Option<SphereTemplate>,
    pub sphere_mesh: Option<Gm<InstancedMesh, PhysicalMaterial>>,
}


fn max_f32(arr: &[f32; 3]) -> f32 {
    arr.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
}

impl Scene {
    /// Creates a new `Scene` instance. context: &Context, initial_viewport: Viewport,
    pub fn new(window: &Window,  scene_settings: SceneSetup)->Self{
        
        let sim_box_size = scene_settings.sim_box.sim_box_size;
        let context = window.gl();
        let initial_viewport = window.viewport();
        let camera = create_camera(initial_viewport, &scene_settings);
        let control = create_control(&camera);
        let simbox = create_simbox(&context, &scene_settings.sim_box);
        let light = create_light(&context);

        let sphere_template = Some(create_sphere_template(&context));

        Self {
            context: context.clone(),
            camera,
            control,
            simbox,
            light,
            sphere_template,
            sphere_mesh: None,
        }
    }

    /// Updates the scene state and renders a single frame.
    /// Takes the current state of particles from the simulation.
    pub fn update_and_render_frame(&mut self, frame_input: &mut FrameInput, particles: &[Particle]) -> FrameOutput { 
        self.camera.set_viewport(frame_input.viewport);

        let Scene {
            ref mut camera,
            ref mut control,
            ..
        } = *self;

        control.handle_events(camera, &mut frame_input.events);

        let screen = frame_input.screen();

        screen
            .clear(ClearState::color_and_depth(0.8, 0.8, 0.8, 1.0, 1.0));

        // Check if the sphere template exists and if there are spheres to render
        if let Some(template) = &self.sphere_template {
            if !particles.is_empty() {
                let transformations: Vec<Mat4> = particles
                    .iter()
                    .map(|p| {
                        Mat4::from_translation(vec3(p.position.x as f32, p.position.y as f32, p.position.z as f32))
                        * Mat4::from_scale(p.radius as f32)
                    })
                    .collect();
                let colors: Vec<Srgba> = particles
                    .iter()
                    .map(|p| p.color)
                    .collect();

                let instances = Instances {
                    transformations,
                    texture_transformations: None,
                    colors: Some(colors),
                };

                self.sphere_mesh = Some(Gm::new(
                    InstancedMesh::new(&self.context, &instances, &template.cpu_mesh),
                    template.material.clone(),
                ));
            }else{
                self.sphere_mesh = None;
            }
        }

        //-------------------------------------------------------------------------------------------
        // Add all the objects to the frame to be rendered
        //-------------------------------------------------------------------------------------------

        let mut main_scene_objects: Vec<&dyn Object> = Vec::with_capacity(particles.len()+1);

        // Add the simulation box if it exists
        if let Some(sim_box_ref) = &self.simbox {
            main_scene_objects.push(sim_box_ref);
        }
        
        // Add the spheres if they exist
        if let Some(sphere_mesh_ref) = &self.sphere_mesh {
            main_scene_objects.push(sphere_mesh_ref);
        }
        
        screen.render(
            &self.camera,
            main_scene_objects,
            &[&self.light],
        );

        FrameOutput::default()
    }

}
