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

    pub fn dummy()->Self{
        Self::new(
            0, // ID doesn't matter for this dummy
            cgmath::Point3::new(8.0, 8.0, 4.0), // Initial position
            cgmath::Vector3::new(0.0, 0.0, 0.0), // Initial velocity
            three_d::core::Srgba::new(255, 0, 0, 255), // Initial color (e.g., opaque red)
            0.5, // Initial radius
        )
    }
}

// No Simulation struct, no update logic, no other functions here.
// This crate now only defines what a Particle is.
// --- ABSOLUTE MINIMUM SIMULATION STRUCT ---
// This version compiles but does not produce any particles or simulate movement.
pub struct Simulation {
    // We only need a particles vector to satisfy the get_particles() method's return type.
    pub particles: Vec<Particle>,
    pub fixed_time_step: f64,
    // fixed_time_step and sim_box_size are no longer needed if update does nothing.
}

impl Simulation {
    // 'new' takes the arguments but does nothing with them, creating an empty particle list.
    pub fn new(_num_particles: usize, _sim_box_size: [f64;3]) -> Self {
        Simulation { particles: Vec::new(), fixed_time_step: 0.01}
    }

    // 'update' does nothing. Particles will not move.
    pub fn update(&mut self) {
        // No code here
    }

    // 'get_particles' returns an empty slice, so md_viz will render nothing from the simulation.
    pub fn get_particles(&self) -> Vec<Particle>{
        vec![Particle::dummy()]
    }
}
// --- END ABSOLUTE MINIMUM SIMULATION STRUCT ---
