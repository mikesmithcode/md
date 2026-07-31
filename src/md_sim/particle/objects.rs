
use three_d::Srgba;
use serde::{Serialize, Deserialize};
use glam::{DVec3, DQuat};


#[derive(Debug, Clone, Copy)]
pub enum ObjectSpec{
    HollowBox(BoxSpec),
    WireBox(BoxSpec),
}

impl ObjectSpec{
    ///Returns a reference to the underlying spec e.g BoxSpec
    pub fn get_spec(&self) -> BoxSpec {
        match self {
            ObjectSpec::HollowBox(boxspec) => *boxspec,
            ObjectSpec::WireBox(boxspec) => *boxspec,
        }
    }
}


///------------------------------------------------------------------------------
/// BoxSpec
/// 
/// This is the configuration of a box on the simulation side. It is rendered
/// in md_viz by a BoxRenderable in md_viz::objects.rs
///------------------------------------------------------------------------------
/// Configuration for a generic box-like object in the scene.
/// 
/// Fields:
/// 
/// visible - turn graphic display on and off
/// thickness - is internal if negative but external if positive
/// position - this coord sets the centre of the box. The axis of system is 0,0,0 in bottom, left, back corner
/// box_size - dimensions. The axis of system is x across, y front-back, z up down 
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct BoxSpec {
    pub visible: bool,
    pub thickness: f64, 
    pub position: DVec3,
    #[serde(skip)]
    pub box_size: DVec3,
    #[serde(skip)]
    pub orientation: DQuat,
    #[serde(skip)]
    pub color: Srgba,
}

impl Default for BoxSpec {
    fn default() -> Self {
        Self {
            visible: true,
            thickness: 0.1,
            position: DVec3::ZERO,
            box_size: DVec3::new(10.0, 0.1, 10.0),
            orientation: DQuat::IDENTITY,
            color: Srgba::WHITE,
        }
    }
}

impl BoxSpec {
    pub fn new(box_size: DVec3, thickness: f64) -> Self {
        Self {
            box_size,
            thickness,
            ..Default::default()
        }
    }
}
