//! scene.rs
//!
//! This module is responsible for drawing everything either to a live window or a video stream.
//! It uses a unified rendering pipeline to ensure visual consistency across all outputs.

use three_d::*;
use three_d::window::HeadlessContext;
use soa_derive::soa_zip;
use std::path::PathBuf;

use winit::window::Window as WinitWindow;
use winit::window::WindowBuilder;
use winit::event_loop::EventLoop;
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::event::{Event as WinitEvent, WindowEvent};

use crate::md_viz::objects::{create_ambient_light, create_directional_light, SimBox, create_simbox};
use crate::md_viz::templates::SphereTemplate;
use crate::md_viz::camera::{create_camera, CameraControl, CameraView};
use crate::md_viz::video::VideoExporter;
use crate::md_sim::particle::ParticleVec;

type RenderableGeometry = Gm<InstancedMesh, PhysicalMaterial>;

#[derive(Debug, Clone)]
pub struct SceneSetup {
    pub camera: CameraView,
    pub window_size: (u32, u32),
    pub vid_fps: u32,
    pub sim_box_setup: SimBox,
}

struct GpuResources {
    ambient_light: AmbientLight,
    directional_light: DirectionalLight,
    simbox: Option<Gm<BoundingBox, PhysicalMaterial>>,
    sphere_template: SphereTemplate,
    particle_mesh: Gm<InstancedMesh, PhysicalMaterial>,
    instance_transforms: Vec<Mat4>,
    instance_colors: Vec<Srgba>,
}

pub enum ContextOwner {
    Window(WindowedContext),
    Headless(HeadlessContext),
}

pub struct Scene {
    settings: SceneSetup,
    pub camera: Camera,
    pub camera_control: CameraControl,

    // Unified Graphics State
    context: Option<Context>,
    owner: Option<ContextOwner>,
    resources: Option<GpuResources>,
    
    // Window Specific
    winit_window: Option<WinitWindow>,
    frame_input_generator: Option<FrameInputGenerator>,
    
    // Headless Specific Persistent Buffers
    headless_target: Option<(Texture2D, DepthTexture2D)>,
    
    video_exporter: Option<VideoExporter>,
    current_frame_pixels: Option<Vec<u8>>,
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
            owner: None,
            resources: None,
            winit_window: None,
            frame_input_generator: None,
            headless_target: None,
            video_exporter: None,
            current_frame_pixels: None,
        }
    }

    fn init_headless(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let h_context = HeadlessContext::new()?;
        let context_handle = (*h_context).clone();

        self.context = Some(context_handle.clone());
        self.owner = Some(ContextOwner::Headless(h_context));

        let (w, h) = self.settings.window_size;

        // Now use self.context.as_ref().unwrap() or context_handle to build textures
        let mut color_texture = Texture2D::new_empty::<[u8; 4]>(
            &context_handle, w, h,
            Interpolation::Nearest, Interpolation::Nearest,
            None, Wrapping::ClampToEdge, Wrapping::ClampToEdge,
        );
        let mut depth_texture = DepthTexture2D::new::<f32>(
            &context_handle, w, h,
            Wrapping::ClampToEdge, Wrapping::ClampToEdge,
        );

        let render_target = RenderTarget::new(color_texture.as_color_target(None),depth_texture.as_depth_target());

        // Initialise resources
        self.resources = Some(Self::_init_gpu_resources(&context_handle, self.settings.sim_box_setup.clone())?);
        self.headless_target = Some((color_texture, depth_texture));
        
        Ok(())
    }

    /// Setup live window and GPU resources
    pub fn init_window(&mut self, event_loop: &EventLoop<()>) -> Result<(), Box<dyn std::error::Error>> {
        let window = WindowBuilder::new()
            .with_title("Simulation")
            .with_inner_size(winit::dpi::PhysicalSize::new(self.settings.window_size.0, self.settings.window_size.1))
            .build(event_loop)?;

        let w_context = WindowedContext::from_winit_window(&window, SurfaceSettings::default())?;
    
        // 1. Save the "Owner" immediately into the struct enum
        let context_handle = (*w_context).clone();
        self.context = Some(context_handle.clone());
        self.owner = Some(ContextOwner::Window(w_context)); 
        self.winit_window = Some(window);

        // 2. NOW initialize resources using the handle that is safely backed by an owner
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
    fn render_to_target(camera: &Camera,resources: &mut GpuResources,target: &mut RenderTarget,particles: &ParticleVec,context: &Context) -> Result<(), Box<dyn std::error::Error>> {
        target.clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0));

        let mut transforms = std::mem::take(&mut resources.instance_transforms);
        let mut colors = std::mem::take(&mut resources.instance_colors);
        transforms.clear();
        colors.clear();

        for (pos, rad, col) in soa_zip!(particles, [position, radius, color]) {
            transforms.push(
                Mat4::from_translation(vec3(pos.x as f32, pos.y as f32, pos.z as f32)) 
                * Mat4::from_scale(*rad as f32) // Radii changes are handled here
            );
            colors.push(*col); // Colour changes are handled here
        }

        let instances = Instances {
            transformations: transforms,
            texture_transformations: None,
            colors: Some(colors),
        };

        // SPEED BOOST: Update the existing GPU buffers instead of re-allocating
        // Note: Since 'resources' is usually shared, you may need a interior mutability 
        // or just pass the mesh in as &mut Gm<InstancedMesh, ...>
        resources.particle_mesh.set_instances(&instances);

        resources.instance_transforms = instances.transformations;
        resources.instance_colors = instances.colors.unwrap();

        let lights: Vec<&dyn Light> = vec![&resources.ambient_light, &resources.directional_light];

        let mut objects: Vec<&dyn Object> = Vec::with_capacity(2);
        objects.push(&resources.particle_mesh);

        if let Some(ref sb) = resources.simbox {
            objects.push(sb); // Pushing the reference directly
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
            
        Self::render_to_target(&self.camera,resources,&mut target,particles,context)?;
        if self.video_exporter.is_some() {
            let pixels = target.read_color::<u8>();
            self.current_frame_pixels = Some(pixels);
        }


        if let Some(ContextOwner::Window(w_ctx)) = &self.owner {
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
        if self.context.is_none() {
            println!("--- No active window detected. Initialising headless context... ---");
            self.init_headless()?; 
        }

        self.video_exporter = Some(VideoExporter::new(path, &self.settings)?);
        Ok(())
    }


    /// Capture the current state to the video exporter
    pub fn save_frame(&mut self, particles: &ParticleVec) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut exporter) = self.video_exporter {
            let (w, h) = self.settings.window_size;
            
            // 1. Extract immutable pieces (Context and Resources)
            let context = self.context.as_ref().ok_or("No context")?;
            let resources = self.resources.as_mut().ok_or("No resources")?;

            // 2. Create the target (Mutably borrowing the texture field)
            let mut target = if self.winit_window.is_some() {
                RenderTarget::screen(context, w, h)
            } else {
                let (color, depth) = self.headless_target.as_mut().ok_or("Headless buffer missing")?;
                RenderTarget::new(color.as_color_target(None), depth.as_depth_target())
            };

            // 3. The Call: Use 'Self::' and pass references
            Self::render_to_target(&self.camera,resources,&mut target, particles,context)?;

            // 4. Export
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
