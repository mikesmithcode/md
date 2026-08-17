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



    pub struct Scene {
        scene_settings: SceneSettings,
        pub camera: Camera,
        pub camera_control: CameraControl,

        // Unified Graphics State
        context: Option<Context>,
        windowed_context: Option<WindowedContext>, // Replaces ContextOwner enum
        resources: Option<GpuResources>,
        
        // Window State
        winit_window: Option<WinitWindow>,
        frame_input_generator: Option<FrameInputGenerator>,
        
        video_exporter: Option<VideoExporter>,
    }

impl Scene {
    ///------------------------------------------------------------------
    /// Setup
    /// -----------------------------------------------------------------
    pub fn new(scene_settings: SceneSettings) -> Self {
        let (w, h) = scene_settings.window_size;
        let viewport = Viewport::new_at_origo(w, h);
        let camera = create_camera(viewport, scene_settings.clone());
        let camera_control = CameraControl::new(&camera, Vector3::new(0.0, 0.0, 0.0));

        Self {
            scene_settings,
            camera,
            camera_control,
            context: None,
            windowed_context: None,
            resources: None,
            winit_window: None,
            frame_input_generator: None,
            video_exporter: None,
        }
    }

    /// Creates a scene by reading a config file and applying simulation overrides
    pub fn from_config(scene_config_path: PathBuf, sim_settings: &SimulationSettings) -> Self {
        let mut settings: SceneSettings = Self::load_json(scene_config_path).unwrap_or_default();
        println!("Scene Settings \n\n {:?}", settings);

        // Update the box_size from simulation
        settings.sim_box.box_size = sim_settings.sim_box_size;
        settings.sim_box.position = sim_settings.sim_box_size*0.5;

        Self::new(settings)
    }

    fn load_json(path: PathBuf) -> Result<SceneSettings, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        Ok(serde_json::from_reader(reader)?)
    }

    /// Initialises the window.
    pub fn view(&mut self, event_loop: &EventLoop<()>) -> Result<(), Box<dyn std::error::Error>> {
        self.init_window(event_loop, true)
    }

    /// Initialises a hardware-accelerated background context (Invisible Window)
    pub fn background(&mut self, event_loop: &EventLoop<()>) -> Result<(), Box<dyn std::error::Error>> {
        self.init_window(event_loop, false)
    }

    // Setup live window and GPU resources
    fn init_window(&mut self, event_loop: &EventLoop<()>, visible: bool) -> Result<(), Box<dyn std::error::Error>> {
        let window = WindowBuilder::new()
            .with_title("Simulation")
            .with_inner_size(winit::dpi::PhysicalSize::new(self.scene_settings.window_size.0, self.scene_settings.window_size.1))
            .with_visible(visible)
            .build(event_loop)?;

        let w_context = WindowedContext::from_winit_window(&window, SurfaceSettings::default())?;
    
        let context_handle = (*w_context).clone();
        self.context = Some(context_handle.clone());
        self.windowed_context = Some(w_context);
        self.winit_window = Some(window);

        let resources = self._init_gpu_resources(&context_handle)?;
        self.resources = Some(resources);
        
        self.frame_input_generator = Some(FrameInputGenerator::from_winit_window(self.winit_window.as_ref().unwrap()));
        
        Ok(())
    }


    ///-----------------------------------------------------------------------------------
    /// Controlling rendering of graphics
    /// ----------------------------------------------------------------------------------
    
    // creates and stores the initial graphic templates for rendering
    fn _init_gpu_resources(&self, context: &Context) -> Result<GpuResources, Box<dyn std::error::Error>> { 
        let simbox_template= WireBoxTemplate::new(context, self.scene_settings.sim_box);//WireBox
        let sphere_template = SphereTemplate::new(context);
        let object_templates = vec![];

        let resources = GpuResources { 
            ambient_light: create_ambient_light(context), 
            directional_light: create_directional_light(context),
            simbox_template,
            sphere_template,
            object_templates,
            instance_transforms: Vec::with_capacity(1000),
            instance_colors: Vec::with_capacity(1000)
        };
        Ok(resources)
    }

    // uses the particle data to transform the graphic template instances
    fn update_particle_instances(camera: &Camera, resources: &mut GpuResources, particles: &ParticleVec) {
        let mut transforms = std::mem::take(&mut resources.instance_transforms);
        let mut colors = std::mem::take(&mut resources.instance_colors);
        transforms.clear();
        colors.clear();
        
        let needs_sorting = particles.color.iter().any(|c| c.a < 255);

        if needs_sorting {
            let cam_pos = camera.position();
            let mut indices: Vec<usize> = (0..particles.len()).collect();
            indices.sort_by(|&a, &b| {
                let pos_a = vec3(particles.position[a].x as f32, particles.position[a].y as f32, particles.position[a].z as f32);
                let pos_b = vec3(particles.position[b].x as f32, particles.position[b].y as f32, particles.position[b].z as f32);
                let dist_a = cam_pos.distance2(pos_a);
                let dist_b = cam_pos.distance2(pos_b);
                dist_b.partial_cmp(&dist_a).unwrap()
            });

            for i in indices {
                resources.sphere_template.push_transform_and_color(i, particles, &mut transforms, &mut colors);
            }
        } else {
            for (pos, rad, col) in soa_zip!(particles, [position, radius, color]) {
                transforms.push(Mat4::from_translation(vec3(pos.x as f32, pos.y as f32, pos.z as f32)) * Mat4::from_scale(*rad as f32));
                colors.push(*col);
            }
        }

        let instances = Instances {
            transformations: transforms,
            texture_transformations: None,
            colors: Some(colors),
        };

        resources.sphere_template.mesh.set_instances(&instances);
        resources.instance_transforms = instances.transformations;
        resources.instance_colors = instances.colors.unwrap();
    }


    /// Central rendering logic used by both display() and save_frame()
    fn render_to_target(context: &Context,
        camera: &Camera,
        resources: &mut GpuResources,
        target: &mut RenderTarget,
        particles: &ParticleVec,
        objects: Option<&[ObjectSpec]>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        target.clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0));

        // update particle graphics instances
        Self::update_particle_instances(camera, resources, particles);

        // Handle dynamic creation or recreation if object specs change / are first loaded
        if let Some(specs) = objects {
            if resources.object_templates.len() != specs.len() {
                resources.object_templates = specs
                    .iter()
                    .map(|spec| match spec {
                        ObjectSpec::WireBox(boxspec) => ObjectTemplate::WireBox(WireBoxTemplate::new(&context, *boxspec)),
                        ObjectSpec::Rectangle(rectspec) => ObjectTemplate::Rectangle(RectTemplate::new(&context, *rectspec)),
                        ObjectSpec::Triangle(trispec) => ObjectTemplate::Triangle(TriTemplate::new(&context, *trispec)),
                    })
                    .collect();
            }
        } else if !resources.object_templates.is_empty() {
            // Clear templates if specs became None
            resources.object_templates.clear();
        }

        // Gather renderable objects dynamically using a vector collection
        let mut scene_objects: Vec<&dyn Object> = Vec::new();

        // Push particles mesh
        scene_objects.push(&resources.sphere_template.mesh);

        
        // Add additional scene objects if present, updating their transforms/colors as needed
        if let Some(specs) = objects {
            for (template, spec) in resources.object_templates.iter_mut().zip(specs.iter()) {
                match template {
                    ObjectTemplate::Rectangle(t) => {
                        t.push_transform_and_color(spec);
                        scene_objects.push(&t.mesh);
                        //println!("Rendering object type at index, total objects: {}", scene_objects.len());
                    }
                    ObjectTemplate::Triangle(t) => {
                        t.push_transform_and_color(spec);
                        scene_objects.push(&t.mesh);
                    }
                    ObjectTemplate::WireBox(t) => {
                        t.push_transform_and_color(spec);
                        scene_objects.push(&t.mesh);
                    }
                }
            }
        }
        
        //Display simulation box outline
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
    /// 
    /// Refresh the live window
    pub fn display(&mut self, particles: &ParticleVec, objects: Option<&[ObjectSpec]>) -> Result<(), Box<dyn std::error::Error>> {
        let context = self.context.as_ref().ok_or("No context")?;
        let resources = self.resources.as_mut().ok_or("No resources")?;
        let generator = self.frame_input_generator.as_mut().ok_or("Not in windowed mode")?;
                
        let frame_input = generator.generate(context);
        self.camera.set_viewport(frame_input.viewport);

        let mut target = RenderTarget::screen(context, frame_input.viewport.width, frame_input.viewport.height);
        
        Self::render_to_target(context, &self.camera,resources,&mut target,particles, objects)?;

        if let Some(w_ctx) = &self.windowed_context {
            w_ctx.swap_buffers()?;
        }

        Ok(())
    }

    pub fn start_recording(&mut self, path: &PathBuf, step: usize) -> Result<(), Box<dyn std::error::Error>> {
        //Format the step with 10-digit padding
        let step_suffix = format!("_{:010}", step);

        //Create the new path with the suffix
        let mut new_path = path.clone();
        
        // Extract current filename without extension
        if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
            let filename_string = format!("{}{}.mp4", file_stem, step_suffix);
            new_path.set_file_name(filename_string);
        } else {
            new_path.push(format!("video{}.mp4", step_suffix));
        }

        self.video_exporter = Some(VideoExporter::new(&new_path, &self.scene_settings)?);
        
        Ok(())
    }

    /// Capture the current state to the video exporter
    pub fn save_frame(&mut self, particles: &ParticleVec, objects: Option<&[ObjectSpec]>) -> Result<(), Box<dyn std::error::Error>> {
        let context = self.context.as_ref().ok_or("No context")?;

        if let Some(ref mut exporter) = self.video_exporter {
            let (w, h) = self.scene_settings.window_size;
            
            let context = self.context.as_ref().ok_or("No context")?;
            let resources = self.resources.as_mut().ok_or("No resources")?;

            let mut target = RenderTarget::screen(context, w, h);

            Self::render_to_target(context, &self.camera,resources,&mut target, particles, objects)?;

            exporter.write_frame(&target.read_color::<[u8; 4]>())?;
        }
        Ok(())
    }

    pub fn close(&mut self) {
        if let Some(exporter) = self.video_exporter.take() {
            let _ = exporter.close();
        }
    }

    ///----------------------------------------------------------------------
    /// Interacting with the window
    /// ---------------------------------------------------------------------
    /// 
    /// Poll events and update camera control
    pub fn poll_events(&mut self, event_loop: &mut EventLoop<()>) -> bool {
        let mut close_requested = false;

        event_loop.run_return(|event, _, control_flow| {
            match event {
                WinitEvent::WindowEvent { event, window_id } => {
                    println!("Window event {:?}", &event);
                    if let Some(window) = &self.winit_window {
                        if window.id() == window_id {
                            self.camera_control.handle_event(&event);

                            if let WindowEvent::CloseRequested = event {
                                close_requested = true;
                                *control_flow = winit::event_loop::ControlFlow::Exit;
                                return;
                            }
                        }
                    }
                }
                _ => {}
            }

            // Stop after processing current events so poll_events can return
            *control_flow = winit::event_loop::ControlFlow::Exit;
        });

        if self.camera_control.update{
            let current_target = self.camera.target().clone();
            self.camera_control.update_camera(&mut self.camera, current_target);
            self.camera_control.update=false;
        }
        close_requested
    }

}
