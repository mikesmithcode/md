use serde::{Serialize, Deserialize};

/// SimulationModel defines the structure of the file to be read in which may be different in different simulations
/// 
/// The json tells serde what variant it should use.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SimulationModel{
    Solid(CollisionParams),
    Fluid {viscosity: f64, cutoff: f64},
    Active(ActiveParams),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ActiveParams {
    pub stiffness: f64,
    pub damping: f64,
    pub Dt: f64,
    pub v0: f64,
    pub gamma: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CollisionParams {
    pub stiffness: f64,
    pub damping: f64,
}
