    //! scene.rs
    //!
    //! This module is responsible for drawing everything either to a live window (WindowedContext) or to a saved png file (HeadlessContext).
    //! 
    //! ## Setup
    //! 
    //! The WindowedContext is supplied externally from an winit EventLoop. Create the event_loop and pass a reference to initialise the window.
    //! 
    //! let mut event_loop = EventLoop::new(); 
    //! let _ = scene.init_window(&event_loop);
    //! 
    //! The HeadlessContext is started without an event loop
    //! 
    //! let _ = scene.init_headless();
    //! 
    //! Create a Scene::new() by passing a clone of the scene_settings. 
    //! 
    //! ## Internal logic
    //! 
    //! render_particles() is called internally by display() or save_img() to draw particles.
    //! poll_events() in live mode looks for changes by the user to the camera view and updates the display.


    use std::collections::HashMap;
    use std::path::Path;

    use three_d::*;  
    use three_d::window::HeadlessContext;
    use three_d::{Srgba, Context, FrameInputGenerator};

    use soa_derive::soa_zip;

    use winit::window::Window as WinitWindow;
    use winit::window::WindowBuilder;
    use winit::event_loop::EventLoop;
    use winit::platform::run_return::EventLoopExtRunReturn;
    use winit::event::{Event as WinitEvent, WindowEvent};

    use crate::objects::{create_ambient_light, create_directional_light};
    use crate::templates::SphereTemplate;
    use crate::objects::{SimBox, create_simbox};
    use crate::camera::{create_camera, CameraControl, CameraSettings};
    use md_core::particle::ParticleVec; 

    use image::{ImageBuffer, Rgba};


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

            if self.camera_control.update{
                let current_target = self.camera.target().clone();
                self.camera_control.update_camera(&mut self.camera, current_target);
                self.camera_control.update=false;
            }
            close_requested
        }

        /// Apply camera control updates
        pub fn update_camera(&mut self) {
            let current_target = self.camera.target().clone();
            self.camera_control.update_camera(&mut self.camera, current_target);
        }

        /// Render to window
        pub fn display(&mut self, particles: &ParticleVec) -> Result<(), Box<dyn std::error::Error>> {
            let generator = self.frame_input_generator.as_mut()
                .expect("Frame generator not setup. Call init_window");
            let context = self.windowed_context.as_ref()
                .expect("Windowed context not initialised");

            let frame_input = generator.generate(context);
            let viewport = frame_input.viewport;
            self.camera.set_viewport(viewport);

            let mut target = RenderTarget::screen(&context, viewport.width, viewport.height);
            target.clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0));

            let resources = self.windowed_resources.as_ref().expect("Windowed gpu resources fail");
            self.render_particles(&context, resources, &mut target, particles)?;

            context.swap_buffers()?;
            Ok(())
        }

        /// Save a headless image
        pub fn save_img(&mut self, particles: &ParticleVec, output_path: &'static str, index: usize) -> Result<(), Box<dyn std::error::Error>> {
            let filename = Path::new(output_path).join("imgs").join( format!("img{:010}.png", index));

            let (frame_width, frame_height) = self.settings.window_size;

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
            img_target.clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0));

            let resources = self.headless_resources.as_ref().expect("Headless gpu resources fail");
            self.render_particles(headless_context, &resources, &mut img_target, particles)?;

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
        /// 
        /// draw_particles uses the templates defined in template.rs and transforms / translates them.
        fn render_particles(
            &self, 
            context: &Context,
            resources: &GpuResources,
            target: &mut RenderTarget, 
            particles: &ParticleVec,
        ) -> Result<(), Box<dyn std::error::Error>> {
        
        
        // 1. Extract data from SoA directly into Vecs for three-d
        // Using with_capacity prevents the Vec from re-allocating as it grows
        let mut transforms = Vec::with_capacity(particles.len());
        let mut colors = Vec::with_capacity(particles.len());

        // This is the high-performance bit: iterating over columns, not objects
        for (pos, rad, col) in soa_zip!(particles, [position, radius, color]) {
            transforms.push(
                Mat4::from_translation(three_d::vec3(pos.x as f32, pos.y as f32, pos.z as f32)) 
                * Mat4::from_scale(*rad as f32)
            );
            colors.push(*col);
        }

        // 2. Create the dynamic mesh from the template
        let sphere_cpu = &resources.sphere_template.as_ref()
            .ok_or("Sphere template missing")?.cpu_mesh;
        
        // This mesh lives only for the duration of this function call
        let particle_mesh = self.create_instanced_mesh(context, sphere_cpu, transforms, colors);

        // 3. Collect everything to be drawn
        let mut objects: Vec<&dyn Object> = Vec::new();
        objects.push(&particle_mesh);
        
        if let Some(sb) = &resources.simbox {
            objects.push(sb);
        }

        // 4. Render to the target
        let lights: Vec<&dyn Light> = vec![
            resources.ambient_light.as_ref().unwrap(),
            resources.directional_light.as_ref().unwrap()
        ];

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



