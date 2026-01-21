use std::collections::HashMap;
use std::path::Path;

use three_d::*;  
use three_d::window::HeadlessContext;
use three_d::{Srgba, Context, FrameInputGenerator};

use winit::window::Window as WinitWindow;
use winit::window::WindowBuilder;
use winit::event_loop::EventLoop;
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::event::{Event as WinitEvent, WindowEvent};

use crate::objects::{create_ambient_light, create_directional_light};
use crate::templates::{Geometry,SphereTemplate};
use crate::objects::{SimBox, create_simbox};
use crate::camera::{create_camera, CameraControl, CameraSettings};
use md_core::particle::Particle;
use image::{ImageBuffer, Rgba};
use crate::Draw;


type RenderableGeometry = Gm<InstancedMesh,PhysicalMaterial>;

/// Support both headless and windowed rendering
pub enum RenderContext {
    Headless(HeadlessContext),
    Windowed(Window),
}

#[derive(Debug, Clone)]
pub struct SceneSetup {
    pub camera: CameraSettings,
    pub window_size: (u32, u32),
    pub sim_box_setup: SimBox,
}

struct GpuResources {
    ambient_light: Option<AmbientLight>,
    directional_light: Option<DirectionalLight>,
    simbox: Option<Gm<BoundingBox, PhysicalMaterial>>,
    sphere_template: Option<SphereTemplate>,
pub instanced_meshes: HashMap<Geometry, RenderableGeometry>,
}

pub struct Scene {
    settings: SceneSetup,
    pub camera: Camera,
    pub camera_control: CameraControl,

    // Windowed resources
    winit_window: Option<WinitWindow>,
    windowed_context: Option<WindowedContext>,
    windowed_resources: Option<GpuResources>,
    frame_input_generator: Option<FrameInputGenerator>,
    
    // Headless resources
    headless_context: Option<HeadlessContext>,
    headless_resources: Option<GpuResources>,
    color_texture: Option<Texture2D>,
    depth_texture: Option<DepthTexture2D>,
}

impl Scene {
    pub fn new(scene_settings: SceneSetup) -> Scene {
        let frame_width = scene_settings.window_size.0;
        let frame_height = scene_settings.window_size.1;

        let viewport = Viewport::new_at_origo(frame_width, frame_height);
        let camera = create_camera(viewport, scene_settings.clone());
        let camera_control = CameraControl::new(&camera, Vector3::new(0.0, 0.0, 0.0));

        Self {
            settings: scene_settings,
            camera,
            camera_control,
            winit_window: None,
            windowed_context: None,
            windowed_resources: None,
            headless_context: None,
            headless_resources: None,            
            frame_input_generator: None,
            color_texture: None,
            depth_texture: None,
        }
    }

    /// Create GPU-backed resources (lights, simbox, sphere template)
    fn _init_gpu_resources(&mut self, context: &Context, sim_box_settings: SimBox) 
        -> Result<GpuResources, Box<dyn std::error::Error>> 
    {
        let resources = GpuResources { 
            ambient_light: Some(create_ambient_light(context)), 
            directional_light: Some(create_directional_light(context)), 
            simbox: create_simbox(context, sim_box_settings), 
            sphere_template: Some(SphereTemplate::new(context)),
            instanced_meshes: HashMap::new(),
        };
        Ok(resources)
    }   

    /// Initialise headless rendering
    pub fn init_headless(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let context = HeadlessContext::new()?;
        self.headless_resources = Some(self._init_gpu_resources(&context, self.settings.sim_box_setup)?);
        self.headless_context = Some(context);
        Ok(())
    }

    /// Setup live window and GPU resources
    pub fn init_window(&mut self, event_loop_ref: &EventLoop<()> ) -> Result<(), Box<dyn std::error::Error>> {
        let winit_window = WindowBuilder::new()
            .with_title("Simulation")
            .with_inner_size(winit::dpi::LogicalSize::new(self.settings.window_size.0, self.settings.window_size.1))
            .build(event_loop_ref)?;

        let context = WindowedContext::from_winit_window(&winit_window, SurfaceSettings::default())?; 
        self.windowed_resources = Some(self._init_gpu_resources(&context, self.settings.sim_box_setup)?);
        self.windowed_context = Some(context);
        self.frame_input_generator = Some(three_d::FrameInputGenerator::from_winit_window(&winit_window));
        self.winit_window = Some(winit_window);
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

        close_requested
    }

    /// Apply camera control updates
    pub fn update_camera(&mut self) {
        self.camera_control.update_camera(&mut self.camera);
    }

    /// Render to window
    pub fn display(&mut self, particles: &[Particle]) -> Result<(), Box<dyn std::error::Error>> {
        let generator = self.frame_input_generator.as_mut()
            .expect("Frame generator not setup. Call init_window");
        let context = self.windowed_context.as_ref()
            .expect("Windowed context not initialised");

        let frame_input = generator.generate(context);
        let mut target = RenderTarget::screen(&context, frame_input.window_width, frame_input.window_height);
        target.clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0));

        let resources = self.windowed_resources.as_ref().expect("Windowed gpu resources fail");
        self.render_particles_to_target(&context, resources, &mut target, particles)?;

        context.swap_buffers()?;
        Ok(())
    }

    /// Save a headless image
    pub fn save_img(&mut self, particles: &[Particle], output_path: &'static str, index: usize) -> Result<(), Box<dyn std::error::Error>> {
        let filename = Path::new(output_path).join("imgs").join( format!("img{:010}.png", index));

        let frame_width = self.settings.window_size.0;
        let frame_height = self.settings.window_size.1;

        let headless_context = self.headless_context.as_ref()
            .expect("Headless context not initialised");

        let mut color_texture = Texture2D::new_empty::<[u8; 4]>(
            &headless_context, frame_width, frame_height, Interpolation::Nearest, Interpolation::Nearest,
            None, Wrapping::ClampToEdge, Wrapping::ClampToEdge,
        );

        let mut depth_texture = DepthTexture2D::new::<f32>(
            &headless_context, frame_width, frame_height, Wrapping::ClampToEdge, Wrapping::ClampToEdge,
        );

        let mut img_target = RenderTarget::new(
            color_texture.as_color_target(None),
            depth_texture.as_depth_target(),
        );

        let resources = self.headless_resources.as_ref().expect("Headless gpu resources fail");
        self.render_particles_to_target(headless_context, &resources, &mut img_target, particles)?;

        let pixels: Vec<u8> = img_target.read_color::<[u8;4]>().into_iter().flatten().collect();
        let image_buffer: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(frame_width, frame_height, pixels)
            .expect("Failed to create ImageBuffer");

        if let Some(parent) = filename.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        image_buffer.save(&filename)?;
        println!("Saved image to {}", std::path::absolute(&filename)?.display());
        Ok(())
    }

    /// Shared rendering logic
    fn render_particles_to_target(
        &self, 
        context: &Context,
        resources: &GpuResources,
        target: &mut RenderTarget, 
        particles: &[Particle]
    ) -> Result<(), Box<dyn std::error::Error>> {
    
    

    // Collection for all the transformations and colors of particles split by geometry type
    //according to the Geometry enum stored in Struct
    type InstanceGroups = HashMap<Geometry, (Vec<Mat4>, Vec<Srgba>)>;
    let mut groups: InstanceGroups = HashMap::new();
    
    let mut particle_geometries: Vec<RenderableGeometry> = Vec::new();

    //Get info about all particles storing colors and transformations by geometry type.
    for p in particles {
        let components = p.get_components()?; 
        
        //There could be multiple things to render from each particle if it is composite.
        for component in components {
            let (transforms, colors) = groups
            .entry(component.template)
            .or_insert_with(|| (Vec::new(), Vec::new()));
            
            transforms.push(component.transformation);
            colors.push(component.color);
        }      
    }


        for (key, (transforms, colors)) in groups {
            let cpu_mesh: &CpuMesh = match key {
                Geometry::Sphere => &resources.sphere_template.as_ref().ok_or("Sphere template missing")?.cpu_mesh,           
            };

        
            let mesh = self.create_instanced_mesh(
                context,
                cpu_mesh,
                transforms, // Note: The Vecs are moved out of the HashMap here
                colors,
            );
        

            particle_geometries.push(mesh);
        }


        //This is a vec to which I will add all objects to be rendered in the scene
        let mut objects: Vec<&dyn three_d::Object> = Vec::new();
        
        //Add particle mesh refs
        for mesh in &particle_geometries {
            objects.push(mesh);
        }
        
        //Add simulation box
        if let Some(simbox) = &resources.simbox {
            objects.push(simbox);
        }
        
        let lights: Vec<&dyn three_d::Light> = vec![
            resources.ambient_light.as_ref().expect("Error creating ambient light"),
            resources.directional_light.as_ref().expect("Error creating directional light")
        ];

        

        target.clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0));
        target.render(&self.camera, objects, &lights);
        Ok(())
    }

    /// Creates a single InstancedMesh object from batched transformation and color data.
    fn create_instanced_mesh(
        &self,
        context: &Context,
        cpu_mesh_template: &CpuMesh, // This replaces the sphere_template lookup
        transformations: Vec<Mat4>,  // Takes the batched transformations
        colors: Vec<Srgba>,          // Takes the batched colors
    ) -> RenderableGeometry {
        
        // --- YOUR REUSED CODE GOES HERE ---
        
        let instances = Instances {
            transformations, // Used directly
            texture_transformations: None,
            colors: Some(colors), // Used directly
        };

        let mut mat = PhysicalMaterial::default();
        mat.albedo = Srgba::WHITE;
        mat.metallic = 0.0;
        mat.roughness = 1.0;

        // This section is slightly modified to use the passed-in template (cpu_mesh_template)
        let instanced_mesh = InstancedMesh::new(context, &instances, cpu_mesh_template); 
        
        // The final result
        Gm::new(instanced_mesh, mat)

    }
}



