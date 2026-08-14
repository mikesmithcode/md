//! # md_viz - graphics for simulation
//!
//! The md_viz folder handles all the graphics related parts of the simulation.
//! The central part of the graphics is the [`Scene`]` struct which coordinates everything.
//! We use [Winit](https://rust-windowing.github.io/winit/winit/index.html) to produce the window and create events. 
//! All the graphics use the rust module three-d which in turn uses open gl (see links below)
//! 
//! 
//! ```rust
//!    let mut scene: Scene = Scene::from_config(scene_config_path, &sim_settings);  
//!    let mut event_loop = EventLoop::new(); 
//!    let _ = scene.view(&event_loop);
//!    let _ = scene.start_recording(&video_path, start_step);
//! ```
//! 
//! During the simulation loop you can then get a reference from the simulation to the particles and objects and render these either as 
//! a display in the window or into a video.
//! 
//! ```rust
//! //Only update every dump frames
//! if step % sim.settings.dump == 0 {
//!       // exit if window close requested
//!       if scene.poll_events(&mut event_loop) {
//!           break; 
//!       }
//!            
//!       //Handle graphics
//!       scene.display(sim.get_particles(), sim.get_objects()).expect("Error updating display");
//!       let _ = scene.save_frame(sim.get_particles(), sim.get_objects());
//! }
//! ```
//! 
//! Recording of video is done using FFMPEG
//! 
//! ## Further details
//! 
//! [`crate::md_viz::cameras_lights_info`]
//! 
#![doc = include_str!("../../docs/opengl.md")]





pub mod camera;
pub mod lights;
pub mod scene;
pub mod scene_settings;
pub mod templates;
pub mod video;
    

pub use self::scene::Scene;
pub use self::scene_settings::SceneSettings;
pub use self::templates::{SphereTemplate, ObjectTemplate, RectTemplate, WireBoxTemplate};

