//! scene.rs
//!
//! This module is responsible for drawing everything either to a live window or a video stream.
//! It uses a unified rendering pipeline to ensure visual consistency across all outputs.

use serde::{Serialize, Deserialize};
use three_d::*;

use crate::md_sim::BoxSpec;
use crate::md_sim::utils::file_io::SimulationPaths;
use crate::md_sim::SimulationSettings;
use crate::md_sim::utils::file_io::load_scene_settings;

use crate::md_viz::camera::MyCamera;
use crate::md_viz::templates::{SphereTemplate, WireBoxTemplate, ObjectTemplate};


/// Configuration parameters governing rendering window dimensions, capture frame rates, camera view settings, and simulation boundary display boxes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)] 
pub struct SceneSettings {
    pub camera: MyCamera,
    pub window_size: (u32, u32),
    pub window_visible: bool,
    pub vid_fps: u32,
    #[serde(default)]
    pub sim_box: BoxSpec,
}

impl SceneSettings {
    pub fn new(sim_paths: &SimulationPaths, sim_settings: &SimulationSettings)-> SceneSettings{
        let mut scene_settings: SceneSettings = load_scene_settings(&sim_paths).unwrap_or_default();

        // Update the box_size from simulation
        scene_settings.sim_box.box_size = sim_settings.sim_box_size;
        scene_settings.sim_box.centre = sim_settings.sim_box_size * 0.5;

        scene_settings
    }
}

impl Default for SceneSettings {
    fn default() -> Self {
        Self {
            camera: MyCamera::default(),
            window_size: (1280, 960),
            window_visible: true,
            vid_fps: 30,
            sim_box: BoxSpec::default(), // The sim_box_size will be overwritten with values from the Simulation config.
        }
    }
}

pub struct GpuResources {
    pub ambient_light: AmbientLight,
    pub directional_light: DirectionalLight,
    pub simbox_template: WireBoxTemplate,
    
    // Separate sphere templates for opaque and transparent instances
    pub opaque_sphere_template: SphereTemplate,
    pub transparent_sphere_template: SphereTemplate,
    
    pub object_templates: Vec<ObjectTemplate>,
    
    // Instance buffers split by transparency mode
    pub opaque_instance_transforms: Vec<Mat4>,
    pub opaque_instance_colours: Vec<Srgba>,
    pub transparent_instance_transforms: Vec<Mat4>,
    pub transparent_instance_colours: Vec<Srgba>,
}
