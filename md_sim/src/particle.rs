// md_sim/src/lib.rs

// These are necessary for defining the Particle's fields
// Make sure nalgebra is a dependency in md_sim/Cargo.toml
use cgmath::{Point3, Vector3};
// Make sure three_d is a dependency in md_sim/Cargo.toml (at least core feature)
use three_d::core::Srgba;


// The Particle struct: This is the only thing md_viz needs from md_sim for now.
#[derive(Debug, Clone, PartialEq)] // Adding these for convenience, not strictly minimum for draw
pub struct Particle {
    pub id: usize,
    pub position: Point3<f64>,  // Position using nalgebra
    pub velocity: Vector3<f64>, // Velocity using nalgebra (can be dummy data for now)
    pub color: Srgba,           // Color using three_d's Srgba
    pub radius: f64,            // Radius as a floating-point number
}

impl Particle {
    pub fn new(id: usize, position: Point3<f64>, velocity: Vector3<f64>, color: Srgba, radius: f64) -> Self {
        Particle { id, position, velocity, color, radius }
    }

    pub fn update(&mut self, dt:f64){
        self.position += self.velocity * dt;

}
}
