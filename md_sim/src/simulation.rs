use md_core::particle::Particle;

use glam::DVec3;
use three_d::core::Srgba;

///---------------------------------------------------------
///Simulation settings 
/// 
/// These are parameters that affect the running of the simulation such as time step.
#[derive(Clone, Copy, Debug)]
pub struct SimulationSettings{
    pub dt: f64,
    pub sim_box_size: [f64; 3], 
    pub start: usize,
    pub stop: usize,
}

impl SimulationSettings{
    pub fn sim_box_size_f32(&self)->[f32;3]{
        [self.sim_box_size[0] as f32,self.sim_box_size[1] as f32,self.sim_box_size[2] as f32]
    }
}
///---------------------------------------------------------

///----------------------------------------------------------
///Simulation
/// 
/// A simulation requires simulation settings (eg timestep, simbox size) a Vec of particles
pub struct Simulation {
    // We only need a particles vector to satisfy the get_particles() method's return type.
    pub particles: Vec<Particle>,
    pub settings: SimulationSettings,
}

impl Simulation {
    /// creates initial position of particles.
    pub fn new(settings: SimulationSettings)-> Self {
        let particles = vec![
            Particle::new(
                0,
                DVec3::new(1.5, 1.5, 1.5),
                DVec3::new(0.0, 0.02, 0.0),
                Srgba::new(255, 0, 0, 255), // Red
                0.5,
            ),
            Particle::new(
                1,
                DVec3::new(1.5, -1.5, -1.5),
                DVec3::new(0.0, 0.003, 0.0),
                Srgba::new(0, 255, 0, 255), // Green
                0.5,
            ),
            Particle::new(
                2,
                DVec3::new(1.5, -1.5, 1.5),
                DVec3::new(0.0, 0.01, 0.0),
                Srgba::new(0, 0, 255, 255), // Blue
                0.5,
            ),
        ];
        Self { particles, settings}
    }

    ///advance the simulation one step
    pub fn update(&mut self) {
        for particle in self.particles.iter_mut() {
            // Update position
            particle.update(self.settings.dt);
        }
    }

    /// 'get_particles' returns an empty slice, so md_viz will render nothing from the simulation.
    pub fn get_particles(&self) -> &Vec<Particle>{
        &self.particles
    }
}

