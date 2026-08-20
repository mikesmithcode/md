//! scene.rs
//!
//! This module is responsible for drawing everything either to a live window or a video stream.
//! It uses a unified rendering pipeline to ensure visual consistency across all outputs.

use std::path::PathBuf;
use three_d::*;
use soa_derive::soa_zip;

use winit::window::Window as WinitWindow;
use winit::window::WindowBuilder;
use winit::event_loop::EventLoop;
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::event::{Event as WinitEvent, WindowEvent};

use crate::md_sim::SimulationSettings;
use crate::md_sim::particle::{ParticleVec, ObjectSpec};

use crate::md_viz::lights::{create_ambient_light, create_directional_light};
use crate::md_viz::templates::{SphereTemplate, RectTemplate, TriTemplate, WireBoxTemplate, ObjectTemplate};
use crate::md_viz::camera::{create_camera, CameraControl};
use crate::md_viz::video::VideoExporter;
use crate::md_viz::SceneSettings;
use crate::md_viz::scene_settings::GpuResources;

/// Manages the rendering context, live window, camera controls, GPU resources, and video export pipelines.
pub struct Scene {
    scene_settings: SceneSettings,
    pub camera: Camera,
    pub camera_control: CameraControl,

    // Unified Graphics State
    context: Context,
    windowed_context: WindowedContext, 
    resources: GpuResources,
    
    // Window State
    winit_window: WinitWindow,
    frame_input_generator: FrameInputGenerator,
    
    video_exporter: Option<VideoExporter>,
}

impl Scene {
    ///------------------------------------------------------------------
    /// Setup
    /// -----------------------------------------------------------------
    
    /// Initializes a new `Scene` instance, loading scene configurations from file, setting up the camera, 
    /// window context, and allocating necessary GPU rendering resources.
    pub fn new(event_loop: &EventLoop<()>, particles: &ParticleVec, objects: Option<&[ObjectSpec]>, scene_config_path: PathBuf, sim_settings: &SimulationSettings) -> Self {
        let mut scene_settings: SceneSettings = Self::load_json(scene_config_path).unwrap_or_default();
        println!("Scene Settings \n\n {:?}", scene_settings);

        // Update the box_size from simulation
        scene_settings.sim_box.box_size = sim_settings.sim_box_size;
        scene_settings.sim_box.center = sim_settings.sim_box_size * 0.5;
        
        let (w, h) = scene_settings.window_size;
        let viewport = Viewport::new_at_origo(w, h);
        let camera = create_camera(viewport, scene_settings.clone());
        let camera_control = CameraControl::new(&camera, Vector3::new(0.0, 0.0, 0.0));

        let (winit_window, windowed_context, context, resources, frame_input_generator) = Scene::init_window(event_loop, &scene_settings, particles, objects);

        Self {
            scene_settings,
            camera,
            camera_control,
            context,
            windowed_context,
            resources,
            winit_window,
            frame_input_generator,
            video_exporter: None,
        }
    }

    // Used to load Scene config from a JSON file.
    fn load_json(path: PathBuf) -> Result<SceneSettings, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        Ok(serde_json::from_reader(reader)?)
    }

    // Sets up the live winit window, three-d context, and GPU resources.
    fn init_window(event_loop: &EventLoop<()>, scene_settings: &SceneSettings, particles: &ParticleVec, objects: Option<&[ObjectSpec]>) -> (winit::window::Window, WindowedContext, Context, GpuResources, FrameInputGenerator) {
        let (w, h) = scene_settings.window_size;

        let window = WindowBuilder::new()
            .with_title("Simulation")
            .with_inner_size(winit::dpi::PhysicalSize::new(w, h))
            .with_visible(scene_settings.window_visible)
            .build(event_loop)
            .expect("Failed to build winit window");

        let w_context = WindowedContext::from_winit_window(&window, SurfaceSettings::default()).expect("Failed to create three-d WindowedContext");
        let context = (*w_context).clone();

        let resources = Self::_init_gpu_resources(&context, particles, objects, scene_settings).expect("Failed to initialize GPU resources");
        
        let frame_input_generator = FrameInputGenerator::from_winit_window(&window);
        
        (window, w_context, context, resources, frame_input_generator)
    }

    ///-----------------------------------------------------------------------------------
    /// Controlling rendering of graphics
    /// ----------------------------------------------------------------------------------
    /// 
    /// Updates or rebuilds object templates and color buffers without rendering anything.
    /// This should be called if anything graphical in your objects or particles changes 
    /// (e.g., particles created or destroyed, colors changed). 
    /// 
    /// # Note
    /// Changes in particle sizes do not require this method, but changes to simulation objects do.
    pub fn update_templates(&mut self, particles: &ParticleVec, objects: Option<&[ObjectSpec]>) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Particle colours
        let mut colours = std::mem::take(&mut self.resources.instance_colours);
        colours.clear();
        for col in &particles.colour {
            colours.push(*col);
        }
        self.resources.instance_colours = colours;

        // 2. Object templates & colours
        if let Some(specs) = objects {
            if self.resources.object_templates.len() != specs.len() {
                self.resources.object_templates = specs
                    .iter()
                    .map(|spec| match spec {
                        ObjectSpec::WireBox(boxspec) => ObjectTemplate::WireBox(WireBoxTemplate::new(&self.context, *boxspec)),
                        ObjectSpec::Rectangle(rectspec) => ObjectTemplate::Rectangle(RectTemplate::new(&self.context, *rectspec)),
                        ObjectSpec::Triangle(trispec) => ObjectTemplate::Triangle(TriTemplate::new(&self.context, *trispec)),
                    })
                    .collect();
            }

            // Update colours/materials for each object template
            for (template, spec) in self.resources.object_templates.iter_mut().zip(specs.iter()) {
                match template {
                    ObjectTemplate::Rectangle(t) => t.update_colour(spec),
                    ObjectTemplate::Triangle(t) => t.update_colour(spec),
                    ObjectTemplate::WireBox(t) => t.update_colour(spec),
                }
            }
        } else {
            // Activates when the last object is removed
            if !self.resources.object_templates.is_empty() {
                self.resources.object_templates.clear();
            }
        }

        Ok(())
    }
    
    // Creates and stores the initial graphic templates for rendering.
    fn _init_gpu_resources(context: &Context, particles: &ParticleVec, objects: Option<&[ObjectSpec]>, scene_settings: &SceneSettings) -> Result<GpuResources, Box<dyn std::error::Error>> { 
        let simbox_template = WireBoxTemplate::new(context, scene_settings.sim_box);
        let sphere_template = SphereTemplate::new(context, particles);

        let count = particles.len();
        let mut instance_transforms = Vec::with_capacity(count);
        let mut instance_colours = Vec::with_capacity(count);

        for i in 0..count {
            sphere_template.push_transform(i, particles, &mut instance_transforms);
            sphere_template.push_colour_and_visibility(i, particles, &mut instance_colours);
        }
        
        // Generate all object templates
        let mut object_templates = vec![];
        if let Some(objs) = objects {
            for object in objs {
                match *object {
                    ObjectSpec::Rectangle(rectspec) => object_templates.push(ObjectTemplate::Rectangle(RectTemplate::new(context, rectspec))),
                    ObjectSpec::Triangle(trispec) => object_templates.push(ObjectTemplate::Triangle(TriTemplate::new(context, trispec))),
                    ObjectSpec::WireBox(boxspec) => object_templates.push(ObjectTemplate::WireBox(WireBoxTemplate::new(context, boxspec))),
                }
            }
        }

        let resources = GpuResources { 
            ambient_light: create_ambient_light(context), 
            directional_light: create_directional_light(context),
            simbox_template,
            sphere_template,
            object_templates,
            instance_transforms,
            instance_colours,
        };

        Ok(resources)
    }

    fn update_particle_transforms(camera: &Camera, resources: &mut GpuResources, particles: &ParticleVec) {
        let mut transforms = std::mem::take(&mut resources.instance_transforms);
        let mut colours = std::mem::take(&mut resources.instance_colours);
        transforms.clear();
        colours.clear();
        
        for (pos, rad, col) in soa_zip!(particles, [position, radius, colour]) {
                transforms.push(Mat4::from_translation(vec3(pos.x as f32, pos.y as f32, pos.z as f32)) * Mat4::from_scale(*rad as f32));
                colours.push(*col);
            }
        /*
        let needs_sorting = particles.colour.iter().any(|c| c.a < 255);

        if needs_sorting {
            let cam_pos = camera.position();
            let mut indices: Vec<usize> = (0..particles.len()).collect();
            indices.sort_by(|&a, &b| {
                let pos_a = vec3(particles.position[a].x as f32, particles.position[a].y as f32, particles.position[a].z as f32);
                let pos_b = vec3(particles.position[b].x as f32, particles.position[b].y as f32, particles.position[b].z as f32);
                let dist_a = cam_pos.distance2(pos_a);
                let dist_b = cam_pos.distance2(pos_b);
                dist_b.partial_cmp(&dist_a).unwrap_or(std::cmp::Ordering::Equal)
            });
            
            for i in indices {
                resources.sphere_template.push_transform(i, particles, &mut transforms);
                colours.push(particles.colour[i]); // Keep colors locked to the exact sorted index order!
            }
        } else {
            for (pos, rad, col) in soa_zip!(particles, [position, radius, colour]) {
                transforms.push(Mat4::from_translation(vec3(pos.x as f32, pos.y as f32, pos.z as f32)) * Mat4::from_scale(*rad as f32));
                colours.push(*col);
            }
        }
        */
        resources.instance_transforms = transforms;
        resources.instance_colours = colours;
    }


    // Updates fast-changing per-frame object transformation properties.
    fn update_object_transforms(
        resources: &mut GpuResources,
        objects: &[ObjectSpec],
    ) {
        for (template, spec) in resources.object_templates.iter_mut().zip(objects.iter()) {
            match template {
                ObjectTemplate::Rectangle(t) => t.update_transform(spec),
                ObjectTemplate::Triangle(t) => t.update_transform(spec),
                ObjectTemplate::WireBox(t) => t.update_transform(spec),
            }
        }
    }

    // Renders the current scene state (particles, objects, simulation box, and lights) onto a target.
    fn render_to_target(
        camera: &Camera,
        resources: &mut GpuResources,
        target: &mut RenderTarget,
        particles: &ParticleVec,
        objects: Option<&[ObjectSpec]>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        target.clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0));

        // 1. Particle updates
        Self::update_particle_transforms(camera, resources, particles);
        //Self::update_particle_colours(resources, particles);

        let instances = Instances {
            transformations: resources.instance_transforms.clone(),
            texture_transformations: None,
            colors: Some(resources.instance_colours.clone()),
        };
        resources.sphere_template.mesh.set_instances(&instances);

        // 3. Object transform updates (run every frame)
        if let Some(objects) = objects {
            Self::update_object_transforms(resources, objects);
        }

        // 4. Gather renderable objects dynamically
        let mut scene_objects: Vec<&dyn Object> = Vec::new();
        scene_objects.push(&resources.sphere_template.mesh);

        for template in &resources.object_templates {
            match template {
                ObjectTemplate::Rectangle(t) => scene_objects.push(&t.mesh),
                ObjectTemplate::Triangle(t) => scene_objects.push(&t.mesh),
                ObjectTemplate::WireBox(t) => scene_objects.push(&t.mesh),
            }
        }

        // Display simulation box outline
        if resources.simbox_template.boxspec.visible {
            scene_objects.push(&resources.simbox_template.mesh);
        }

        // Setup lights and execute draw call
        let lights: Vec<&dyn Light> = vec![&resources.ambient_light, &resources.directional_light];
        target.render(camera, scene_objects, &lights);
        
        Ok(())
    }

    ///-------------------------------------------------------------------------------------------
    /// Outputs to window and file
    /// ------------------------------------------------------------------------------------------
    
    /// Refreshes and renders the current frame to the live window.
    pub fn display(&mut self, particles: &ParticleVec, objects: Option<&[ObjectSpec]>) -> Result<(), Box<dyn std::error::Error>> {               
        let frame_input = self.frame_input_generator.generate(&self.context);
        self.camera.set_viewport(frame_input.viewport);

        let mut target = RenderTarget::screen(&self.context, frame_input.viewport.width, frame_input.viewport.height);
        
        Self::render_to_target(&self.camera, &mut self.resources, &mut target, particles, objects)?;

        let _ = self.windowed_context.swap_buffers();

        Ok(())
    }

    /// Initializes a video exporter to begin recording frames to disk with an optional step-based suffix.
    pub fn start_recording(&mut self, path: &PathBuf, step: usize) -> Result<(), Box<dyn std::error::Error>> {
        let step_suffix = format!("_{:010}", step);
        let mut new_path = path.clone();
        
        if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
            let filename_string = format!("{}{}.mp4", file_stem, step_suffix);
            new_path.set_file_name(filename_string);
        } else {
            new_path.push(format!("video{}.mp4", step_suffix));
        }

        self.video_exporter = Some(VideoExporter::new(&new_path, &self.scene_settings)?);
        
        Ok(())
    }

    /// Captures the current rendered frame and writes it out via the active video exporter.
    pub fn save_frame(&mut self, particles: &ParticleVec, objects: Option<&[ObjectSpec]>) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut exporter) = self.video_exporter {
            let (w, h) = self.scene_settings.window_size;

            let mut target = RenderTarget::screen(&self.context, w, h);

            Self::render_to_target(&self.camera, &mut self.resources, &mut target, particles, objects)?;

            exporter.write_frame(&target.read_color::<[u8; 4]>())?;
        }
        Ok(())
    }

    /// Closes and finalizes the video recording file if an exporter is active.
    pub fn close(&mut self) {
        if let Some(exporter) = self.video_exporter.take() {
            let _ = exporter.close();
        }
    }

    ///----------------------------------------------------------------------
    /// Interacting with the window
    /// ---------------------------------------------------------------------
    
    /// Polls incoming window events, updates camera controls, and returns a boolean indicating whether a close was requested.
    pub fn poll_events(&mut self, event_loop: &mut EventLoop<()>) -> bool {
        let mut close_requested = false;

        event_loop.run_return(|event, _, control_flow| {
            *control_flow = winit::event_loop::ControlFlow::Exit;

            if let WinitEvent::WindowEvent { event, window_id } = event {
                if self.winit_window.id() == window_id {
                    self.camera_control.handle_event(&event);

                    if let WindowEvent::CloseRequested = event {
                        close_requested = true;
                    }
                }
            }
        });

        if self.camera_control.update {
            let current_target = self.camera.target().clone();
            self.camera_control.update_camera(&mut self.camera, current_target);
            self.camera_control.update = false;
        }
        
        close_requested
    }
}