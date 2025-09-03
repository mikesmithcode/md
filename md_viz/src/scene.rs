use three_d::*;
use three_d::window::HeadlessContext;
use three_d::Srgba;
use crate::objects::{create_camera, create_ambient_light, create_directional_light};

use crate::shapes::{SphereTemplate, SimBox, create_simbox};
use md_core::particle::Particle;
use crate::objects::CameraSettings;
use image::{ImageBuffer, Rgba};

pub struct SceneSetup {
    pub camera: CameraSettings,
    pub window_size: (u32, u32),
    pub sim_box: SimBox,
}
pub struct Scene {
    context: HeadlessContext,
    camera: Camera,
    ambient_light: AmbientLight,
    directional_light: DirectionalLight,
    simbox: Option<Gm<BoundingBox, PhysicalMaterial>>,
    sphere_template: SphereTemplate,
    width: u32,
    height: u32,
    // 🐛 Fixed: Use Option types to allow for a two-step initialization
    color_texture: Texture2D,
    depth_texture: DepthTexture2D,
}

impl Scene {
    pub fn new(scene_settings: &SceneSetup) -> Scene {
        let context = HeadlessContext::new().expect("Failed to create context");
        let width = scene_settings.window_size.0;
        let height = scene_settings.window_size.1;

        let viewport = Viewport::new_at_origo(width, height);
        let camera = create_camera(viewport, scene_settings);
        let ambient_light = create_ambient_light(&context);
        let directional_light = create_directional_light(&context);
        
        let simbox = create_simbox(&context, &scene_settings.sim_box);
        let sphere_template = SphereTemplate::new(&context);
        
        let color_texture = Texture2D::new_empty::<[u8; 4]>(
            &context, width, height, Interpolation::Nearest, Interpolation::Nearest, None, Wrapping::ClampToEdge, Wrapping::ClampToEdge,
        );
        
        let depth_texture = DepthTexture2D::new::<f32>(
            &context, width, height, Wrapping::ClampToEdge, Wrapping::ClampToEdge,
        );

        Self {
            context,
            camera,
            ambient_light,
            directional_light,
            simbox,
            sphere_template,
            width,
            height,
            // Initialize with None
            color_texture,
            depth_texture,
        }
    }

    pub fn save_img(&mut self,particles: &[Particle],filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Build RenderTarget temporarily
    let img_target = RenderTarget::new(
        self.color_texture.as_color_target(None),
        self.depth_texture.as_depth_target(),
    );

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
    mat.albedo = Srgba::WHITE;   // base color neutral
    mat.metallic = 0.0;
    mat.roughness = 1.0;
    // One instanced sphere mesh for all particles
    let sphere_mesh = Gm::new(
        InstancedMesh::new(&*self.context, &instances, &self.sphere_template.cpu_mesh), mat
    );

    // Collect objects to render
    let mut objects: Vec<&dyn three_d::Object> = Vec::new();
    if let Some(simbox) = &self.simbox {
        objects.push(simbox);
    }
    objects.push(&sphere_mesh);

    // Render
    img_target.clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0));
    img_target.render(&self.camera, objects, &[&self.ambient_light, &self.directional_light]);

    // Read back pixels
    let pixels: Vec<u8> = img_target.read_color::<[u8;4]>().into_iter().flatten().collect();

    // Create ImageBuffer
    let image_buffer: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(self.width, self.height, pixels)
            .ok_or("Failed to create ImageBuffer")?;

    // Ensure output folder exists
    if let Some(parent) = std::path::Path::new(filename).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Save image
    image_buffer.save(filename)?;
    println!("Saved image to {:?}", std::fs::canonicalize(filename)?);

    Ok(())
}
}
