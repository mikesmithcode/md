    use std::io::ErrorKind;

    use three_d::*;  
    use three_d::window::HeadlessContext;
    use three_d::{Srgba, Context, FrameInputGenerator};
    //use three_d_winit::*;
    use winit::window::Window as WinitWindow;
    use winit::window::WindowBuilder;
    use winit::event_loop::EventLoop;
    use crate::objects::{create_camera, create_ambient_light, create_directional_light};

    use crate::shapes::{SphereTemplate, SimBox, create_simbox};
    use md_core::particle::Particle;
    use crate::objects::CameraSettings;
    use image::{ImageBuffer, Rgba};


    // Add this enum to support both headless and windowed contexts
    pub enum RenderContext {
    Headless(HeadlessContext),
    Windowed(Window),
}


#[derive(Debug, Clone)]
pub struct SceneSetup {
    pub camera: CameraSettings,
    pub window_size: (u32, u32),
    pub sim_box_setup: SimBox,
    pub img_filepath: String,
}

struct GpuResources {
    ambient_light: Option<AmbientLight>,
    directional_light: Option<DirectionalLight>,
    simbox: Option<Gm<BoundingBox, PhysicalMaterial>>,
    sphere_template: Option<SphereTemplate>,
}

pub struct Scene {
    settings: SceneSetup,
    camera: Camera,

    // Core scene data - shared between contexts
    
        
    // Windowed resources
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
        
        Self {
            settings: scene_settings,
            camera,
            windowed_context: None,
            windowed_resources: None,
            headless_context: None,
            headless_resources: None,            
            frame_input_generator: None,
            color_texture: None,
            depth_texture: None,
        }
    }

    /// Private helper to create all GPU-backed resources (lights, simbox, templates).
    /// This is called only if the resources haven't been created yet.
    fn _init_gpu_resources(&mut self, context: &Context, sim_box_settings: SimBox)-> Result<GpuResources, Box<dyn std::error::Error>> {
        let resources = GpuResources { 
                ambient_light: Some(create_ambient_light(context)), 
                directional_light: Some(create_directional_light(context)), 
                simbox: create_simbox(context, sim_box_settings), 
                sphere_template: Some(SphereTemplate::new(context)),
            };

        Ok(resources)
    }   

    ///Initialise headless rendering
    pub fn init_headless(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let context = HeadlessContext::new()?;
        self.headless_resources = Some(self._init_gpu_resources(&context, self.settings.sim_box_setup)?);
        self.headless_context = Some(context);
        Ok(())
    }

    //Setup code for a live window
    pub fn init_window(&mut self, event_loop_ref: &EventLoop<()> ) -> Result<(), Box<dyn std::error::Error>> {
        let winit_window = WindowBuilder::new()
            .with_title("Simulation")
            .with_inner_size(winit::dpi::LogicalSize::new(self.settings.window_size.0, self.settings.window_size.1))
            .build(event_loop_ref).expect("New window failed");

         let context = WindowedContext::from_winit_window(&winit_window, SurfaceSettings::default())?; 
         self.windowed_resources =  Some(self._init_gpu_resources(&context, self.settings.sim_box_setup)?);
         self.windowed_context = Some(context);
         self.frame_input_generator  = Some(three_d::FrameInputGenerator::from_winit_window(&winit_window));
        Ok(())
    }
    
    // Display to window (requires window context)
    pub fn display(&mut self, particles: &[Particle]) -> Result<(), Box<dyn std::error::Error>>{
        let generator = self.frame_input_generator.as_mut().expect("Frame generator not setup. Call init_window");
        let context = self.windowed_context.as_ref().expect("Context requires window to be initialised");
        let frame_input = generator.generate(context);

        // Render directly to the window
        let mut target = RenderTarget::screen(&context, frame_input.window_width, frame_input.window_height);
        target.clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0));
        let resources = self.windowed_resources.as_ref().expect("Windowed gpu resources fail");
        self.render_particles_to_target(&context, &resources, &mut target, particles)?;
        let _=context.swap_buffers()?;
        Ok(())
    }

    // Save image (requires headless context)
    pub fn save_img(&mut self, particles: &[Particle], filestub: &str, index: usize) -> Result<(), Box<dyn std::error::Error>> {

        let filename = &format!("{}{:04}.png", filestub, index);

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

        let mut img_target = {
        RenderTarget::new(
            color_texture.as_color_target(None),
            depth_texture.as_depth_target(),
        )
        };

        let resources = self.headless_resources.as_ref().expect("Headless gpu resources fail");

        let _ = self.render_particles_to_target(headless_context, &resources, &mut img_target, particles)?;

        // Read back pixels and save
        let pixels: Vec<u8> = img_target.read_color::<[u8;4]>().into_iter().flatten().collect();
        
        let image_buffer: ImageBuffer<Rgba<u8>, _> =
            ImageBuffer::from_raw(frame_width, frame_height, pixels)
                .expect("Failed to create ImageBuffer");

        if let Some(parent) = std::path::Path::new(filename).parent() {
            match std::fs::create_dir_all(parent) {
                Ok(_) => {}, // Directory created or already exists: success!
                Err(e) if e.kind() == ErrorKind::AlreadyExists => {}, 
                Err(e) => return Err(Box::new(e)),
    }
        }

        image_buffer.save(filename).expect("Img writing failed");
        println!("Saved image to {:?}", std::fs::canonicalize(filename)?);
        Ok(())
    }


    // Shared rendering logic that works with any context
    fn render_particles_to_target(
        &self, 
        context: &Context,
        resources: &GpuResources,
        target: &mut RenderTarget, 
        particles: &[Particle]
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Build instancing data from particles (convert DVec3 to Vec3)
        // Build instancing data from particles
        let transformations: Vec<Mat4> = particles
            .iter()
            .map(|p| {
                Mat4::from_translation(vec3(
                    p.position.x as f32,
                    p.position.y as f32,
                    p.position.z as f32,
                )) * Mat4::from_scale(p.radius as f32)
            })
            .collect();

        let colors: Vec<Srgba> = particles.iter().map(|p| p.color).collect();

        let instances = Instances {
            transformations,
            texture_transformations: None,
            colors: Some(colors),
        };

        let mut mat = PhysicalMaterial::default();
        mat.albedo = Srgba::WHITE;
        mat.metallic = 0.0;
        mat.roughness = 1.0;

        // Fixed: Remove unnecessary dereferencing and unwrap
        let sphere_template = resources.sphere_template.as_ref().expect("Sphere template not loaded");
        let sphere_mesh = Gm::new(
            InstancedMesh::new(context, &instances, &sphere_template.cpu_mesh), 
            mat
        );

        // Collect objects to render
        let mut objects: Vec<&dyn three_d::Object> = Vec::new();
        if let Some(simbox) = &resources.simbox {
            objects.push(simbox);
        }
        objects.push(&sphere_mesh);

        // Fixed: Create slice from Vec for lights
        let lights: Vec<&dyn three_d::Light> = vec![
            resources.ambient_light.as_ref().expect("Error creating ambient light"),
            resources.directional_light.as_ref().expect("Error creating directional light")
        ];

        // Render
        target.clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0));
        target.render(&self.camera, objects, &lights);

        Ok(())
    }

   

}


