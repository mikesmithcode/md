//! SimulationModel defines the structure of the file to be read in which may be different in different simulations
//! 
//! The json tells serde what variant it should use.

use serde::{Serialize, Deserialize};


/// A SimulationModel is a way of getting general simulation parameters into the simulation.
/// Things like friction, fluid viscosity or whatever. Add this to your sim_settings.json
/// ```json
/// "model": {
///    "type": "FrictionParams",
///    "stiffness": 66500.0,
///    "damping": 0.3,
///    "mu": 0.4
///  }
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SimulationModel{
    Frictional(FrictionParams)
}




/// This is used for simulations with particle-particle and particle-plane rigid sphere inelastic and frictional interactions.
/// You can of course not use some of these parameters if this is not the case or just set them to 0.0 or whatever's appropriate.
/// If you want to be safe make an enum variant that doesn't have the params you don't need so that functions panic if you use
/// them incorrectly.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FrictionParams{
    pub stiffness: f64,
    pub damping: f64,
    pub mu: f64,
    pub plane_stiffness: f64,
    pub plane_damping: f64,
    pub plane_mu: f64
}

impl Default for FrictionParams{
    fn default()-> Self{
        FrictionParams { stiffness: 50000.0, damping: 1.0, mu: 0.3, plane_stiffness: 50000.0, plane_damping: 1.0, plane_mu: 0.3}
    }
}
