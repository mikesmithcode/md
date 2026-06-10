//! scene.rs
//!
//! This module is responsible for drawing everything either to a live window or a video stream.
//! It uses a unified rendering pipeline to ensure visual consistency across all outputs.

use three_d::*;
use soa_derive::soa_zip;
use std::path::PathBuf;

use winit::window::Window as WinitWindow;
use winit::window::WindowBuilder;
use winit::event_loop::EventLoop;
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::event::{Event as WinitEvent, WindowEvent};

use crate::SimulationSettings;
use crate::md_viz::objects::{create_ambient_light, create_directional_light, SimBox, create_simbox};
use crate::md_viz::templates::SphereTemplate;
use crate::md_viz::camera::{create_camera, CameraControl, CameraView};
use crate::md_viz::video::VideoExporter;
use crate::md_sim::particle::ParticleVec;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)] 
pub struct SceneSetup {
    pub camera: CameraView,
    pub window_size: (u32, u32),
    pub vid_fps: u32,
    pub sim_box_setup: SimBox,
}

impl Default for SceneSetup {
    fn default() -> Self {
        Self {
            camera: CameraView::Perspective,
            window_size: (1280, 960),
            vid_fps: 30,
            sim_box_setup: SimBox::default(),//The sim_box_size will be overwritten with values from the Simulation config.
        }
    }
}

struct GpuResources {
    ambient_light: AmbientLight,
    directional_light: DirectionalLight,
    simbox: Option<Gm<BoundingBox, PhysicalMaterial>>,
    #[allow(dead_code)]
    sphere_template: SphereTemplate,
    particle_mesh: Gm<InstancedMesh, PhysicalMaterial>,
    instance_transforms: Vec<Mat4>,
    instance_colors: Vec<Srgba>,
}


pub struct Scene {
    settings: SceneSetup,
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
    pub fn new(scene_settings: SceneSetup) -> Self {
        let (w, h) = scene_settings.window_size;
        let viewport = Viewport::new_at_origo(w, h);
        let camera = create_camera(viewport, scene_settings.clone());
        let camera_control = CameraControl::new(&camera, Vector3::new(0.0, 0.0, 0.0));

        Self {
            settings: scene_settings,
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
        let mut settings = Self::load_json(scene_config_path).unwrap_or_default();
        println!("Scene Settings \n\n {:?}", settings);
        // Update the sim_box_size with real simulation data
        settings.sim_box_setup.sim_box_size = sim_settings.sim_box_size_f32();

        Self::new(settings)
    }

    fn load_json(path: PathBuf) -> Result<SceneSetup, Box<dyn std::error::Error>> {
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
            .with_inner_size(winit::dpi::PhysicalSize::new(self.settings.window_size.0, self.settings.window_size.1))
            .with_visible(visible)
            .build(event_loop)?;

        let w_context = WindowedContext::from_winit_window(&window, SurfaceSettings::default())?;
    
        let context_handle = (*w_context).clone();
        self.context = Some(context_handle.clone());
        self.windowed_context = Some(w_context);
        self.winit_window = Some(window);

        // Initialize resources using the handle that is safely backed by an owner
        let resources = Self::_init_gpu_resources(&context_handle, self.settings.sim_box_setup.clone())?;
        self.resources = Some(resources);
        
        self.frame_input_generator = Some(FrameInputGenerator::from_winit_window(self.winit_window.as_ref().unwrap()));
        
        Ok(())
    }

    // Remove '&self' from the start
    fn _init_gpu_resources(context: &Context, sim_box_settings: SimBox) -> Result<GpuResources, Box<dyn std::error::Error>> { 
        let sphere_template = SphereTemplate::new(context);
    
        // Create an initial empty mesh
        let mut mat = PhysicalMaterial::default();
        mat.albedo = Srgba::WHITE;
        mat.render_states.blend = Blend::TRANSPARENCY;
        mat.render_states.cull = Cull::Back;

        let particle_mesh = Gm::new(
            InstancedMesh::new(context, &Instances::default(), &sphere_template.cpu_mesh),
            mat
        );

        let resources = GpuResources { 
            ambient_light: create_ambient_light(context), 
            directional_light: create_directional_light(context), 
            simbox: create_simbox(context, sim_box_settings), 
            sphere_template,
            particle_mesh,
            instance_transforms: Vec::with_capacity(1000),
            instance_colors: Vec::with_capacity(1000)
        };
        Ok(resources)
    }

    /// Central rendering logic used by both display() and save_frame()
    fn render_to_target(camera: &Camera,resources: &mut GpuResources,target: &mut RenderTarget,particles: &ParticleVec) -> Result<(), Box<dyn std::error::Error>> {
        target.clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0));

        let mut transforms = std::mem::take(&mut resources.instance_transforms);
        let mut colors = std::mem::take(&mut resources.instance_colors);
        transforms.clear();
        colors.clear();

        for (pos, rel_pos, rad, col) in soa_zip!(particles, [position, rel_pos, radius, color]) {
            transforms.push(
                Mat4::from_translation(vec3((pos.x + rel_pos.x) as f32, (pos.y + rel_pos.y) as f32, (pos.z + rel_pos.z) as f32)) 
                * Mat4::from_scale(*rad as f32) // Radii changes are handled here
            );
            colors.push(*col); // Colour changes are handled here
        }

        let instances = Instances {
            transformations: transforms,
            texture_transformations: None,
            colors: Some(colors),
        };

        // Update the existing GPU buffers
        resources.particle_mesh.set_instances(&instances);

        resources.instance_transforms = instances.transformations;
        resources.instance_colors = instances.colors.unwrap();

        let lights: Vec<&dyn Light> = vec![&resources.ambient_light, &resources.directional_light];

        let mut objects: Vec<&dyn Object> = Vec::with_capacity(2);
        objects.push(&resources.particle_mesh);

        if let Some(ref sb) = resources.simbox {
            objects.push(sb); 
        }
        
        // Render the persistent mesh
        target.render(camera, objects, &lights);
        
        Ok(())
    }

    /// Refresh the live window
    pub fn display(&mut self, particles: &ParticleVec) -> Result<(), Box<dyn std::error::Error>> {
        let context = self.context.as_ref().ok_or("No context")?;
        let resources = self.resources.as_mut().ok_or("No resources")?;
        let generator = self.frame_input_generator.as_mut().ok_or("Not in windowed mode")?;
                
        let frame_input = generator.generate(context);
        self.camera.set_viewport(frame_input.viewport);

        let mut target = RenderTarget::screen(context, frame_input.viewport.width, frame_input.viewport.height);
            
        Self::render_to_target(&self.camera,resources,&mut target,particles)?;

        if let Some(w_ctx) = &self.windowed_context {
            w_ctx.swap_buffers()?;
        }

        Ok(())
    }

    /// Poll events and update camera control
    pub fn poll_events(&mut self, event_loop: &mut EventLoop<()>) -> bool {
        let mut close_requested = false;

        event_loop.run_return(|event, _, control_flow| {
            match event {
                WinitEvent::WindowEvent { event, window_id } => {
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

        self.video_exporter = Some(VideoExporter::new(&new_path, &self.settings)?);
        
        Ok(())
    }


    /// Capture the current state to the video exporter
    pub fn save_frame(&mut self, particles: &ParticleVec) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut exporter) = self.video_exporter {
            let (w, h) = self.settings.window_size;
            
            let context = self.context.as_ref().ok_or("No context")?;
            let resources = self.resources.as_mut().ok_or("No resources")?;

          let mut target = RenderTarget::screen(context, w, h);

            Self::render_to_target(&self.camera,resources,&mut target, particles)?;

            exporter.write_frame(&target.read_color::<[u8; 4]>())?;
        }
        Ok(())
    }

    pub fn close(&mut self) {
        if let Some(exporter) = self.video_exporter.take() {
            let _ = exporter.close();
        }
    }
}
