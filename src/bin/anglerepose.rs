/// Explanation of simulation
/// 
/// Silo consists of a 2D hopper with diagonal walls and a flat bottom. We then drop a square lattice
/// of balls from above into it and watch everything slosh around.


use glam::DVec3;
use std::collections::HashMap;

// Import everything from your md_viz library


// Imports from simulation library
use md::md_sim::{ObjectSpec, Forces, Motion, ParticleVec, SimulationSettings, Interactivity};
use md::md_sim::force::{add_directional_weight, add_particle_particle_collision};
use md::md_sim::motion::{integrate_rigid_bodies, integrate_rigid_bodies_correct};
use md::md_sim::particle::MoleculeData;
use md::md_viz::actions::UserAction;



pub struct SimUpdate{
    up: DVec3,
    angle: f64,
}

impl Interactivity for SimUpdate{

    // Left and Right Arrow tilt the box around y axis.
    fn handle_key(&mut self, key: UserAction) {
        let vertical = glam::DVec3::Z; 
        match key {
                UserAction::Right => {
                    // Decrease angle to rotate clockwise, and invert the Y-axis rotation for 'up'
                    let angle_delta = -1.0_f64.to_radians();
                    let rotation = glam::DQuat::from_axis_angle(glam::DVec3::Y, angle_delta);
                    self.up = rotation * self.up;    
                    self.angle += angle_delta; // Accumulate signed angle
                    println!("Rotated clockwise, angle: {} deg", self.angle.to_degrees());
                }
                UserAction::Left => {
                    // Increase angle to rotate counterclockwise
                    let angle_delta = 1.0_f64.to_radians();
                    let rotation = glam::DQuat::from_axis_angle(glam::DVec3::Y, angle_delta);
                    self.up = rotation * self.up;
                    self.angle += angle_delta; // Accumulate signed angle
                    println!("Rotated counterclockwise, angle: {} deg", self.angle.to_degrees());
                }
                _ => {
                    println!("Unhandled key: {:?}", key);
                }
            }
        }
    }

impl Forces for SimUpdate{
    // Default implementation is true, set to false if not using
    fn has_pair_forces(&self)-> bool {
        true
    }
    // Default implementation is true set to false if not using
    fn has_single_forces(&self)-> bool {
        true
    }

    fn has_object_forces(&self) -> bool {
        false
    }


    //Forces which apply to every particle individually
    fn update_single_forces(&self,i:usize, mut force:glam::DVec3, _torque: DVec3, particles: &ParticleVec, _settings: &SimulationSettings, _time: f64)->(DVec3, DVec3) {   
        // Only the main particle has weight
        if particles.ptype[i] == 0 || particles.ptype[i] == 2{
            let up = self.up;
            force = add_directional_weight(i, force, particles, up);
        }
        (force, _torque)
    }

    // forces that operate between pairs of particles
    fn update_pair_forces(&self,i: usize,j: usize, mut force: DVec3, mut torque: DVec3, particles: &ParticleVec,settings: &SimulationSettings)->(DVec3, DVec3){
        if particles.ptype[i] == 0 || particles.ptype[i] == 2{
            //Only main particles have granular collisions. 
            (force, torque)=add_particle_particle_collision(i, j, particles, force, torque, settings);
        }
        //else{
        //    //ptype == 1 is the charge.
        //    force = add_coulomb(i, j, particles, force, settings);
        //}

    
        (force, torque)
    }

}

impl Motion for SimUpdate{
    fn update_motion(&self, forces: &[glam::DVec3], torques: &[DVec3],particles: &mut ParticleVec,settings: &SimulationSettings, molecule_map: &HashMap<usize, MoleculeData>, _time:f64) {
        integrate_rigid_bodies(forces,torques, particles, molecule_map, settings);
    }

    fn correct_motion(&self, forces: &[glam::DVec3], torques: &[DVec3], particles: &mut ParticleVec,settings: &SimulationSettings, molecule_map: &HashMap<usize, MoleculeData>) {
        integrate_rigid_bodies_correct(forces, torques, particles, molecule_map, settings);
    }


    fn update_objects(&self, object: &mut ObjectSpec, _particles: &mut ParticleVec, settings: &SimulationSettings, time: f64) {
    
        // Provides a line to indicate the slope.
        match object {
            ObjectSpec::Line(line) => {
                //Rotate around midpoint according to angle to indicate tilt.
                let vertex_vec = line.vertices[1] - line.vertices[0];
                let half_len = vertex_vec.length() * 0.5;

                let centre = (line.vertices[1] + line.vertices[0]) * 0.5;

                // 2. Create a properly rotated displacement vector in the XZ plane
                let displacement = DVec3::new(
                    half_len * self.angle.cos(), 
                    0.0, 
                    half_len * self.angle.sin()
                );

                // 3. Compute endpoints maintaining the exact original length and rotation
                let first_vertex = centre + displacement;
                let second_vertex = centre - displacement;

                // 4. Update the line endpoints in place
                line.update_endpoints([first_vertex, second_vertex]);
            },
            _ => {}   
        }
    }

}




pub fn main() {    
    md::run_simulation(SimUpdate {up: DVec3::new(0.0,0.0,1.0), angle: 0.0});
}