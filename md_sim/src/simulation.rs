use crate::particle::Particle;

use cgmath::{Point3, Vector3};
use three_d::core::Srgba;

pub struct SimulationSettings{
    pub dt: f64,
    pub sim_box_size: [f64; 3], 
}

impl SimulationSettings{
    pub fn sim_box_size_f32(&self)->[f32;3]{
        [self.sim_box_size[0] as f32,self.sim_box_size[1] as f32,self.sim_box_size[2] as f32]
    }
}

// No Simulation struct, no update logic, no other functions here.
// This crate now only defines what a Particle is.
// --- ABSOLUTE MINIMUM SIMULATION STRUCT ---
// This version compiles but does not produce any particles or simulate movement.
pub struct Simulation {
    // We only need a particles vector to satisfy the get_particles() method's return type.
    pub particles: Vec<Particle>,
    pub settings: SimulationSettings,
}

impl Simulation {
    // 'new' takes the arguments but does nothing with them, creating an empty particle list.
    pub fn new(settings: SimulationSettings)-> Self {
        let particles = vec![
            Particle::new(
                0,
                Point3::new(2.5, 2.5, 2.5),
                Vector3::new(0.0, 0.02, 0.0),
                Srgba::new(255, 0, 0, 255), // Red
                0.5,
            ),
            Particle::new(
                1,
                Point3::new(2.5, -2.5, -2.5),
                Vector3::new(0.0, 0.003, 0.0),
                Srgba::new(0, 255, 0, 255), // Green
                0.5,
            ),
            Particle::new(
                2,
                Point3::new(2.5, -2.5, 2.5),
                Vector3::new(0.0, 0.01, 0.0),
                Srgba::new(0, 0, 255, 255), // Blue
                0.5,
            ),
        ];
        Self { particles, settings}
    }

    pub fn update(&mut self) {
        for particle in self.particles.iter_mut() {
            // Update position
            particle.update(self.settings.dt);
        }
    }

    // 'get_particles' returns an empty slice, so md_viz will render nothing from the simulation.
    pub fn get_particles(&self) -> &Vec<Particle>{
        &self.particles
    }
}



// md_viz/src/lib.rs (or another file where your visualization code lives)
// This is an adjusted version of the main loop logic we discussed earlier.

