//! scene.rs
//!
//! This module is responsible for drawing everything either to a live window or a video stream.
//! It uses a unified rendering pipeline to ensure visual consistency across all outputs.

use serde::{Serialize, Deserialize};
use three_d::*;

use crate::md_sim::BoxSpec;

use crate::md_viz::camera::MyCamera;
use crate::md_viz::templates::{SphereTemplate, WireBoxTemplate, ObjectTemplate, BoxTemplate};




#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)] 
pub struct SceneSettings {
    pub camera: MyCamera,
    pub window_size: (u32, u32),
    pub vid_fps: u32,
    #[serde(default)]
    pub sim_box: BoxSpec,
}

impl Default for SceneSettings {
    fn default() -> Self {

        Self {
            camera: MyCamera::default(),
            window_size: (1280, 960),
            vid_fps: 30,
            sim_box: BoxSpec::default(),//The sim_box_size will be overwritten with values from the Simulation config.
        }
    }
}



pub struct GpuResources {
    pub ambient_light: AmbientLight,
    pub directional_light: DirectionalLight,
    pub simbox_template: WireBoxTemplate,
    #[allow(dead_code)]
    pub sphere_template: SphereTemplate,//Create instances which are updated starting from a single template
    pub object_templates: Vec<ObjectTemplate>,//Each object gets its own template which is transformed.
    pub instance_transforms: Vec<Mat4>,
    pub instance_colors: Vec<Srgba>,
}