use three_d::*;
use cgmath::{Matrix3, SquareMatrix};
use three_d::core::Mat4;

// Import the Particle and Simulation from your simulation crate
use md_sim::{Particle, Simulation};

// For shared mutable state within the render loop
use std::rc::Rc;
use std::cell::RefCell;

pub use crate::shapes::{create_simbox, create_sphere, SimBox};
pub use crate::objects::{create_window, create_camera, create_control, create_light, create_gizmo, Gizmo, create_axes, CameraSettings};





/*-----------------------------------------------------------------------------------
Create structs
-------------------------------------------------------------------------------------*/
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
    pub gizmo: Gizmo,
}


fn max_f32(arr: &[f32; 3]) -> f32 {
    arr.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
}

impl Scene {
    /// Creates a new `Scene` instance. context: &Context, initial_viewport: Viewport,
    pub fn new(window: &Window,  scene_settings: SceneSetup)->Self{//, num_initial_particles: usize) -> Self {
        
        let sim_box_size = scene_settings.sim_box.sim_box_size_f32();
        let context = window.gl();
        let initial_viewport = window.viewport();
        let sim_box_max: f32 = max_f32(&sim_box_size);
        let camera = create_camera(initial_viewport, scene_settings.camera, sim_box_size);
        let control = create_control(&camera);
        let simbox = create_simbox(&context, scene_settings.sim_box);
        let light = create_light(&context);
        let gizmo = create_gizmo(&context, sim_box_max);

        Self {
            context: context.clone(),
            camera,
            control,
            simbox,
            light,
            gizmo,
        }
    }

    /// Updates the scene state and renders a single frame.
    /// Takes the current state of particles from the simulation.
    pub fn update_and_render_frame(&mut self, frame_input: &mut FrameInput) -> FrameOutput { // , particles: Vec<Particle>
        self.camera.set_viewport(frame_input.viewport);

        let Scene {
            ref mut camera,
            ref mut control,
            ..
        } = *self;

        control.handle_events(camera, &mut frame_input.events);

        let mut screen = frame_input.screen();

        // --- MAIN SCENE RENDERING ---
        screen
            .clear(ClearState::color_and_depth(0.8, 0.8, 0.8, 1.0, 1.0));

        let mut main_scene_objects: Vec<&dyn Object> = match &self.simbox {
            Some(sim_box_ref) => vec![sim_box_ref],
            None => vec![],
        };


        screen.render(
            &self.camera,
            main_scene_objects,
            &[&self.light],
        );

        // --- GIZMO AXES RENDERING ---
        let window_width = frame_input.viewport.width as f32;
        let window_height = frame_input.viewport.height as f32;

        let gizmo_viewport = Viewport {
            x: (window_width - self.gizmo.gizmo_size_pixels - self.gizmo.gizmo_padding_pixels) as i32,
            y: (window_height - self.gizmo.gizmo_size_pixels - self.gizmo.gizmo_padding_pixels) as i32,
            width: self.gizmo.gizmo_size_pixels as u32,
            height: self.gizmo.gizmo_size_pixels as u32,
        };
        self.gizmo.gizmo_camera.set_viewport(gizmo_viewport);

        let main_cam_world_transform: Mat4 = self.camera.view().invert().unwrap();

        let main_cam_rotation_only_transform = Matrix3::new(
            main_cam_world_transform.x.x, main_cam_world_transform.y.x, main_cam_world_transform.z.x,
            main_cam_world_transform.x.y, main_cam_world_transform.y.y, main_cam_world_transform.z.y,
            main_cam_world_transform.x.z, main_cam_world_transform.y.z, main_cam_world_transform.z.z,
        );

        let gizmo_cam_distance = 2.0;
        let gizmo_cam_pos_local = vec3(20.0, 0.0, gizmo_cam_distance);
        let gizmo_cam_pos_world = main_cam_rotation_only_transform.invert().unwrap() * gizmo_cam_pos_local;

        self.gizmo.gizmo_camera.set_view(
            gizmo_cam_pos_world,
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
        );

        screen
            .clear(ClearState::depth(1.0))
            .render(
                &self.gizmo.gizmo_camera,
                &self.gizmo.gizmo_axes,
                &[&self.light],
            );

        FrameOutput::default()
    }

    /// Starts the main application loop, orchestrating simulation and visualization.
    pub fn run(self, window: Window){//, simulation: Simulation) {
        let mut accumulator = 0.0;

        let app_state_rc = Rc::new(RefCell::new(self));

        window.render_loop(move |mut frame_input| {
            let mut app = app_state_rc.borrow_mut();

            app.camera.set_viewport(frame_input.viewport);

            // --- Fixed Time Step Simulation Loop ---
            accumulator += frame_input.elapsed_time;
            let fixed_sim_dt = 0.1;
            while accumulator >= fixed_sim_dt {
                //sim.update();
                accumulator -= fixed_sim_dt;
            }
            // ----------------------------------------

            //let particles_data = sim.get_particles();

            app.update_and_render_frame(&mut frame_input)//, particles_data)
        });
    }
}
