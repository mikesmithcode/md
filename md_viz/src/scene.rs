    use std::io::Error;

use three_d::*;
    use three_d::window::HeadlessContext;
    use three_d::Srgba;
    use winit::event_loop::EventLoop;
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
#[derive(Debug, Clone, Copy)]
pub struct SceneSetup {
    pub camera: CameraSettings,
    pub window_size: (u32, u32),
    pub sim_box: SimBox,
}
pub struct Scene {
    settings: SceneSetup,
    // Core scene data - shared between contexts
    camera: Camera,
    ambient_light: Option<AmbientLight>,
    directional_light: Option<DirectionalLight>,
    simbox: Option<Gm<BoundingBox, PhysicalMaterial>>,
    sphere_template: Option<SphereTemplate>,
    width: u32,
    height: u32,
    
    // Separate contexts
    headless_context: Option<HeadlessContext>,
    window: Option<Window>,
    
    // Headless rendering resources
    color_texture: Option<Texture2D>,
    depth_texture: Option<DepthTexture2D>,
}

impl Scene {
    pub fn new(scene_settings: SceneSetup) -> Scene {
        let width = scene_settings.window_size.0;
        let height = scene_settings.window_size.1;

        let viewport = Viewport::new_at_origo(width, height);
        let camera = create_camera(viewport, scene_settings.clone());
        
        Self {
            settings: scene_settings,
            camera,
            ambient_light: None,
            directional_light: None,
            simbox: None,
            sphere_template: None,
            width,
            height,
            headless_context: None,
            window: None,
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
        self._init_resources(&context, self.settings.sim_box);        
        self.headless_context = Some(context);
        Ok(())
    }
    
    // Initialize window rendering
    pub fn init_window(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let window = Window::new(WindowSettings {
            title: "Particle Simulation".to_string(),
            max_size: Some((self.width, self.height)),
            ..Default::default()
        })?;

        let context = window.gl();
        self._init_resources(&context, self.settings.sim_box);
        self.window = Some(window);
        Ok(())
    }
    

    // Display to window (requires window context)
    pub fn display(&self, particles: &[Particle]) -> Result<(), Box<dyn std::error::Error>>{
        let window = self.window.as_ref().expect("window failed");
        let context = window.gl();
        // Render directly to the window
        let mut target = RenderTarget::screen(&context, self.width, self.height);
        target.clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0));
        self.render_particles_to_target(&context, &mut target, particles)?;
        Ok(())
    }

    // Save image (requires headless context)
    pub fn save_img(&mut self, particles: &[Particle], filename: &str) -> Result<(), Error> {
        let headless_context = self.headless_context.as_ref()
            .expect("Headless context not initialised");

        let mut color_texture = Texture2D::new_empty::<[u8; 4]>(
            &headless_context, self.width, self.height, Interpolation::Nearest, Interpolation::Nearest,
            None, Wrapping::ClampToEdge, Wrapping::ClampToEdge,
        );
        
        let mut depth_texture = DepthTexture2D::new::<f32>(
            &headless_context, self.width, self.height, Wrapping::ClampToEdge, Wrapping::ClampToEdge,
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
            ImageBuffer::from_raw(self.width, self.height, pixels)
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
