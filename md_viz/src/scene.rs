    use std::io::Error;

    use three_d::*;
    
    use three_d::window::HeadlessContext;
    use three_d::{Srgba, Context, FrameInputGenerator};
    //use three_d_winit::*;
    use winit::{event::{Event, WindowEvent}, event_loop::{ControlFlow, EventLoop}, window::WindowBuilder};
    use winit::window::Window as WinitWindow;
    use crate::objects::{create_camera, create_ambient_light, create_directional_light};

    use crate::scene;
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


pub struct Scene {
    settings: SceneSetup,

    // Core scene data - shared between contexts
    camera: Camera,
    ambient_light: Option<AmbientLight>,
    directional_light: Option<DirectionalLight>,
    simbox: Option<Gm<BoundingBox, PhysicalMaterial>>,
    sphere_template: Option<SphereTemplate>,
    
    // Windowed resources
    windowed_context: Option<WindowedContext>,
    frame_input_generator: Option<FrameInputGenerator>,
    
    // Headless resources
    headless_context: Option<HeadlessContext>,
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
            ambient_light: None,
            directional_light: None,
            simbox: None, 
            sphere_template: None,
            headless_context: None,
            windowed_context: None,
            frame_input_generator: None,
            color_texture: None,
            depth_texture: None,
        }
    }

    /// Private helper to create all GPU-backed resources (lights, simbox, templates).
    /// This is called only if the resources haven't been created yet.
    fn _init_resources(&mut self, context: &Context, sim_box_settings: SimBox) {
        if self.ambient_light.is_none() {
            println!("Initializing shared GPU resources...");
            self.ambient_light = Some(create_ambient_light(context));
            self.directional_light = Some(create_directional_light(context));
            self.simbox = create_simbox(context, sim_box_settings);
            self.sphere_template = Some(SphereTemplate::new(context));
        }
    }   

    ///Initialise headless rendering
    pub fn init_headless(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let context = HeadlessContext::new()?;
        self._init_resources(&context, self.settings.sim_box_setup);        
        self.headless_context = Some(context);
        Ok(())
    }

    pub fn init_window(&mut self, winit_window: WinitWindow) -> Result<(), Box<dyn std::error::Error>> {
         let context = WindowedContext::from_winit_window(&winit_window, SurfaceSettings::default())?; 
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
        self.render_particles_to_target(&context, &mut target, particles)?;
        Ok(())
    }

    // Save image (requires headless context)
    pub fn save_img(&mut self, particles: &[Particle], filename: &str) -> Result<(), Error> {

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

        // Pass headless_context directly without double dereferencing
        self.render_particles_to_target(headless_context, &mut img_target, particles);

        // Read back pixels and save
        let pixels: Vec<u8> = img_target.read_color::<[u8;4]>().into_iter().flatten().collect();
        
        let image_buffer: ImageBuffer<Rgba<u8>, _> =
            ImageBuffer::from_raw(frame_width, frame_height, pixels)
                .expect("Failed to create ImageBuffer");

        if let Some(parent) = std::path::Path::new(filename).parent() {
            std::fs::create_dir_all(parent)?;
        }

        image_buffer.save(filename).expect("Img reading failed");
        println!("Saved image to {:?}", std::fs::canonicalize(filename)?);
        Ok(())
    }


    // Shared rendering logic that works with any context
    fn render_particles_to_target(
        &self, 
        context: &Context,
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
        let sphere_template = self.sphere_template.as_ref().unwrap();
        let sphere_mesh = Gm::new(
            InstancedMesh::new(context, &instances, &sphere_template.cpu_mesh), 
            mat
        );

        // Collect objects to render
        let mut objects: Vec<&dyn three_d::Object> = Vec::new();
        if let Some(simbox) = &self.simbox {
            objects.push(simbox);
        }
        objects.push(&sphere_mesh);

        // Fixed: Create slice from Vec for lights
        let lights: Vec<&dyn three_d::Light> = vec![
            self.ambient_light.as_ref().unwrap(),
            self.directional_light.as_ref().unwrap()
        ];

        // Render
        target.clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0));
        target.render(&self.camera, objects, &lights);

        Ok(())
    }

   

}


