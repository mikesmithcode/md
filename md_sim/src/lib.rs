pub mod particle;
pub mod simulation;

pub use crate::particle::*;
pub use crate::simulation::*;


pub fn run_headless_simulation(simulation: &mut Simulation, num_steps: usize) {
    println!("Starting headless simulation...");
    for i in 0..num_steps {
        simulation.update();
        if i % 1000 == 0 {
            println!("Simulating step {}", i);
        }
    }
    println!("Headless simulation finished after {} steps.", num_steps);
}
