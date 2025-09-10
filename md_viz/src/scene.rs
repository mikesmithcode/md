    use three_d::*;
    use three_d::window::HeadlessContext;
    use three_d::Srgba;
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

pub struct SceneSetup {
    pub camera: CameraSettings,
    pub window_size: (u32, u32),
    pub sim_box: SimBox,
}
pub struct Scene {
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
    pub fn new(scene_settings: &SceneSetup) -> Scene {
        let width = scene_settings.window_size.0;
        let height = scene_settings.window_size.1;

        let viewport = Viewport::new_at_origo(width, height);
        let camera = create_camera(viewport, scene_settings);
        let ambient_light = create_ambient_light(&context);
        let directional_light = create_directional_light(&context);
        
        let simbox = create_simbox(&context, &scene_settings.sim_box);
        let sphere_template = SphereTemplate::new(&context);
        

        Self {
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

    ///Initialise headless rendering
    pub fn init_headless(&mut self, scene_settings: &SceneSetup) -> Result<(), Box<dyn std::error::Error>> {
        let context = HeadlessContext::new()?;
        
        // Create resources using headless context
        self.ambient_light = Some(create_ambient_light(&context));
        self.directional_light = Some(create_directional_light(&context));
        self.simbox = create_simbox(&context, &scene_settings.sim_box);
        self.sphere_template = Some(SphereTemplate::new(&context));
        
        // Create textures for off-screen rendering
        self.color_texture = Some(Texture2D::new_empty::<[u8; 4]>(
            &context, self.width, self.height, Interpolation::Nearest, Interpolation::Nearest,
            None, Wrapping::ClampToEdge, Wrapping::ClampToEdge,
        ));
        
        self.depth_texture = Some(DepthTexture2D::new::<f32>(
            &context, self.width, self.height, Wrapping::ClampToEdge, Wrapping::ClampToEdge,
        ));
        
        self.headless_context = Some(context);
        Ok(())
    }

    // Initialize window rendering
    pub fn init_window(&mut self, scene_settings: &SceneSetup) -> Result<(), Box<dyn std::error::Error>> {
        let window = Window::new(WindowSettings {
            title: "Particle Simulation".to_string(),
            max_size: Some((self.width, self.height)),
            ..Default::default()
        })?;

        // If we don't have resources yet, create them using window context
        if self.ambient_light.is_none() {
            self.ambient_light = Some(create_ambient_light(&window.gl()));
            self.directional_light = Some(create_directional_light(&window.gl()));
            self.simbox = create_simbox(&window.gl(), &scene_settings.sim_box);
            self.sphere_template = Some(SphereTemplate::new(&window.gl()));
        }

        self.window = Some(window);
        Ok(())
    }


    

    // Display to window (requires window context)
pub fn display(&mut self, particles: &[Particle]) -> Result<bool, Box<dyn std::error::Error>> {
    let window = self.window.as_mut()
        .ok_or("Window context not initialized. Call init_window() first.")?;

    // Handle events and check for exit
    let mut should_exit = false;
    for event in window.events().iter() {
        match event {
            three_d::Event::WindowCloseRequested => {
                should_exit = true;
            }
            three_d::Event::WindowResized { width, height } => {
                self.width = *width;
                self.height = *height;
                let viewport = three_d::Viewport::new_at_origo(self.width, self.height);
                self.camera.set_viewport(viewport);
            }
            _ => {} // Handle other events as needed
        }
    }
    
    if should_exit {
        return Ok(false);
    }

    // Get the screen render target
    let screen = window.screen();
    
    // Render to the screen
    self.render_particles_to_target(&window.gl(), &screen, particles)?;
    
    // Present the frame (swap buffers)
    window.swap_buffers();
    
    Ok(true) // Continue running
}

    // Save image (requires headless context)
pub fn save_img(&mut self, particles: &[Particle], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let headless_context = self.headless_context.as_ref()
        .ok_or("Headless context not initialized. Call init_headless() first.")?;
    
    let color_texture = self.color_texture.as_mut().unwrap();
    let depth_texture = self.depth_texture.as_mut().unwrap();

    let img_target = RenderTarget::new(
        color_texture.as_color_target(None),
        depth_texture.as_depth_target(),
    );

    // Pass headless_context directly without double dereferencing
    self.render_particles_to_target(headless_context, &img_target, particles)?;

    // Read back pixels and save
    let pixels: Vec<u8> = img_target.read_color::<[u8;4]>().into_iter().flatten().collect();
    
    let image_buffer: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(self.width, self.height, pixels)
            .ok_or("Failed to create ImageBuffer")?;

    if let Some(parent) = std::path::Path::new(filename).parent() {
        std::fs::create_dir_all(parent)?;
    }

    image_buffer.save(filename)?;
    println!("Saved image to {:?}", std::fs::canonicalize(filename)?);
    Ok(())
}

// Shared rendering logic that works with any context
fn render_particles_to_target(
    &self, 
    context: &Context,
    target: &RenderTarget, 
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
