//! motion of the particles in the broadest sense
//! 
//! Each simulation should define a Motion trait which requires you to implement
//! update_motion() which occurs before forces are calculated and correct_motion() which occurs afterwards.


use std::collections::HashMap;

use glam::{DVec3,DMat3, DQuat};
use itertools::izip;
use three_d::Srgba;
//use rand_distr::Distribution;

use crate::md_sim::motion::geometry::calculate_molecule_com;
use crate::md_sim::{simulation::SimulationSettings, particle::ParticleVec};
use crate::md_sim::utils::models::SimulationModel;
use crate::md_sim::motion::geometry::MoleculeData;

/// Defines the integration scheme and kinematic updates for the simulation.
///
/// The `Motion` trait is responsible for advancing the simulation in time. It 
/// handles the numerical integration of Newton's laws of motion, as well as 
/// the application of boundary conditions (e.g., periodic wrapping or wall reflections).
pub trait Motion {
    /// Advances particle states at the start of a simulation step (Prediction).
    ///
    /// This method is typically called before force accumulation. In a standard 
    /// Velocity Verlet scheme, this is used to update positions based on current 
    /// velocities and to perform a "half-step" update to velocities using 
    /// **previous** force data.
    ///
    /// # Arguments
    ///
    /// * `forces` - The force buffer calculated during the **previous** timestep.
    /// * `particles` - The mutable particle data to be updated.
    /// * `settings` - Global simulation parameters, including the timestep ($\Delta t$).
    ///
    /// # Implementation Note
    /// Standalone integration functions (like `verlet_predict`) 
    /// should be called within this method to maintain a modular design.
    fn update_motion(
        &self, 
        _forces: &[DVec3], 
        _torques: &[DVec3],
        _particles: &mut ParticleVec, 
        _settings: &SimulationSettings,
        _molecule_map: &HashMap<usize, MoleculeData>,
        _time: f64
    );

    /// Finalises particle states at the end of a simulation step (Correction).
    ///
    /// This is an optional hook called after the **current** forces have been 
    /// accumulated. It is primarily used in multi-step integrators to correct 
    /// velocities using the newly calculated force data.
    ///
    /// # Default Implementation
    /// The default implementation is empty, making this step optional.
    ///
    /// # Arguments
    ///
    /// * `forces` - The force buffer calculated during the **current** timestep.
    /// * `particles` - The mutable particle data to be corrected.
    /// * `settings` - Global simulation parameters.
    fn correct_motion(
        &self, 
        _forces: &[DVec3], 
        _torques: &[DVec3],
        _particles: &mut ParticleVec, 
        _settings: &SimulationSettings,
        _molecule_map: &HashMap<usize, MoleculeData>
    ) {
        // Optional: No correction by default
    }
}


pub fn update_abps(forces: &[DVec3], particles: &mut ParticleVec, settings: &SimulationSettings) {
    println!("{:?}", forces);
    if let SimulationModel::Active(params) = &settings.model {
        let inv_gamma = 1.0 / params.gamma;
        let mut _rng = rand::thread_rng();
        let _normal = rand_distr::Normal::new(0.0, 1.0).unwrap();

        

        for i in 0..particles.position.len() {
            // Update Linear Velocity and Position (Overdamped)
            particles.velocity[i] = forces[i] * inv_gamma;
            particles.position[i] += particles.velocity[i] * settings.dt;

            // Calculate the scale for rotational noise
            #[allow(non_snake_case)]
            let Dr = 3.0 * params.Dt / (4.0 * particles.radius[i].powi(2));
            let _theta_noise_scale = (2.0 * Dr * settings.dt).sqrt();
            let d_theta = 0.0;//normal.sample(&mut rng) * theta_noise_scale;

            // Apply Rotational Noise safely to the 3D Heading Vector
            // We create a clean rotation quaternion around the Y-axis (up-axis for X-Z plane)
            let rotation = glam::DQuat::from_axis_angle(glam::DVec3::Y, d_theta);
            
            // Rotate the entire orientation vector safely
            particles.orientation[i] = rotation * particles.orientation[i];
            particles.orientation[i] = particles.orientation[i].normalize();

            // Debug Checks
            if particles.position[i].x.is_nan() || particles.position[i].x.abs() > 1e6 {
               println!("Particle exploded! Force: {:?}, Position: {:?}", forces[i], particles.position[i]);
            }
            
            // Apply periodic boundaries
            check_periodic(&mut particles.position[i], settings.sim_box_size);
        }
    }
}



/// Performs the first half of the Velocity Verlet integration for multiparticle rigid bodies (Prediction).
///
/// This function should be called inside `update_motion`. It uses the forces 
/// from the **previous** timestep to:
/// 1. Update velocities by a half-step: $v(t + \frac{\Delta t}{2}) = v(t) + \frac{a(t)\Delta t}{2}$
/// 2. Update positions by a full step: $x(t + \Delta t) = x(t) + v(t + \frac{\Delta t}{2})\Delta t$
///
/// After this call, positions are finalised for the current step, allowing 
/// for new force calculations (e.g., collisions) at $x(t + \Delta t)$.
pub fn integrate_rigid_bodies(
    forces: &[DVec3], 
    torques: &[DVec3],
    particles: &mut ParticleVec, 
    molecule_map: &HashMap<usize, MoleculeData>,
    settings: &SimulationSettings
) {
    let dt = settings.dt;
    let half_dt = dt * 0.5;
    let sim_box_size = settings.sim_box_size;

    for (mol_id, mol) in molecule_map {   
        let lead_idx = mol.pids[0];    
        
        // Calculate current COM etc
        let (total_mass, com_pos, com_vel) = calculate_molecule_com(&mol.pids, &particles);

        // Calculate aggregate forces and torques
        let mut total_force = DVec3::ZERO;
        let mut total_torque = DVec3::ZERO;
        for &idx in &mol.pids {
            total_force += forces[idx];
            let r = particles.position[idx] - com_pos;
            total_torque += torques[idx] + r.cross(forces[idx]);
        }

        // Update COM Velocity and Angular Velocity
        let acc = total_force / total_mass;
        let new_com_vel = com_vel + (acc * half_dt);
        
        let rot_mat = DMat3::from_quat(particles.orientation[lead_idx]);
        let i_global = rot_mat * mol.inertia * rot_mat.transpose();
        let omega = particles.omega[lead_idx];
        let gyroscopic = omega.cross(i_global * omega);
        let alpha = i_global.inverse() * (total_torque - gyroscopic);
        let new_omega = omega + (alpha * half_dt);

        // Update Orientation and COM Position
        let new_com_pos = com_pos + (new_com_vel * dt);
        let delta_q = DQuat::from_scaled_axis(new_omega * dt);
        let new_orientation = (delta_q * particles.orientation[lead_idx]).normalize();
        
        // Update every particle's state
        let rot_mat_new = DMat3::from_quat(new_orientation);
        for &idx in &mol.pids {
            // Update individual velocity: v_i = v_com + (omega x r_global)
            let r_global = rot_mat_new * particles.rel_pos[idx];
            particles.velocity[idx] = new_com_vel + new_omega.cross(r_global);
            
            // Update individual position
            particles.position[idx] = new_com_pos + r_global;
            
            // Sync orientation and omega (if stored per-particle)
            particles.orientation[idx] = new_orientation;
            particles.omega[idx] = new_omega;
        }
    }
}

/// Performs the second half of the Velocity Verlet integration for rigid bodies (Correction).
///
/// This function should be called inside `correct_motion`. It uses the forces 
/// calculated at the **new** positions to finalise the velocities:
/// $v(t + \Delta t) = v(t + \frac{\Delta t}{2}) + \frac{a(t + \Delta t)\Delta t}{2}$
pub fn integrate_rigid_bodies_correct(
    forces: &[DVec3], 
    torques: &[DVec3],
    particles: &mut ParticleVec, 
    molecule_map: &HashMap<usize, MoleculeData>,
    settings: &SimulationSettings
) {
    let half_dt = settings.dt * 0.5;

    for (_m_id, mol) in molecule_map {
        let lead_idx = mol.pids[0];
        
        // Calculate new Force/Torque at the new position
        let (total_mass, com_pos, com_vel) = calculate_molecule_com(&mol.pids, particles);
        let mut total_force = DVec3::ZERO;
        let mut total_torque = DVec3::ZERO;
        for &idx in &mol.pids {
            total_force += forces[idx];
            let r = particles.position[idx] - com_pos;
            total_torque += torques[idx] + r.cross(forces[idx]);
        }

        // Calculate COM velocity (v_new = v_half + a_new * dt/2)
        let acc = total_force / total_mass;
        let new_com_vel = com_vel + (acc * half_dt);

        // Finalise angular velocity (w_new = w_half + alpha_new * dt/2)
        let rot_mat = DMat3::from_quat(particles.orientation[lead_idx]);
        let i_global = rot_mat * mol.inertia * rot_mat.transpose();
        let i_inv = i_global.inverse();
        let omega = particles.omega[lead_idx];
        let gyroscopic = omega.cross(i_global * omega);
        let alpha = i_inv * (total_torque - gyroscopic);
        let new_omega = omega + (alpha * half_dt);

        for &idx in &mol.pids {
            particles.omega[idx] = new_omega;
            // Re-sync all particles with the new COM velocity and new Omega
            let r_global = particles.position[idx] - com_pos;
            particles.velocity[idx] = new_com_vel + new_omega.cross(r_global);
        }
    }
}


/// Performs the first half of the Velocity Verlet integration (Prediction).
///
/// This function should be called inside `update_motion`. It uses the forces 
/// from the **previous** timestep to:
/// 1. Update velocities by a half-step: $v(t + \frac{\Delta t}{2}) = v(t) + \frac{a(t)\Delta t}{2}$
/// 2. Update positions by a full step: $x(t + \Delta t) = x(t) + v(t + \frac{\Delta t}{2})\Delta t$
///
/// After this call, positions are finalised for the current step, allowing 
/// for new force calculations (e.g., collisions) at $x(t + \Delta t)$.
pub fn integrate_singleparticle_update(
    forces: &[DVec3], 
    _torques: &[DVec3],
    particles: &mut ParticleVec, 
    settings: &SimulationSettings
) {
    let dt = settings.dt;
    let half_dt = dt * 0.5;
    let sim_box_size = settings.sim_box_size;

    let _is_rotating = matches!(settings.model, SimulationModel::SolidFriction(_));

    for (pos, vel, _orientation, _omega, &mass, &_inertia, &force, &_torque) in izip!(
        &mut particles.position,
        &mut particles.velocity,
        &mut particles.orientation,
        &mut particles.omega,
        &particles.mass,
        &particles.inertia,
        forces, 
        _torques,
    ) {
        let acceleration = force / mass;
        
        // Half-step velocity update
        *vel += acceleration * half_dt;
        // Full-step position update
        *pos += *vel * dt;
        
        // Enforce boundary conditions
        check_periodic(pos, sim_box_size);
    }
}

/// Performs the second half of the Velocity Verlet integration (Correction).
///
/// This function should be called inside `correct_motion`. It uses the forces 
/// calculated at the **new** positions to finalise the velocities:
/// $v(t + \Delta t) = v(t + \frac{\Delta t}{2}) + \frac{a(t + \Delta t)\Delta t}{2}$
pub fn integrate_singleparticle_correct(
    forces: &[DVec3], 
    torques: &[DVec3],
    particles: &mut ParticleVec, 
    settings: &SimulationSettings
) {
    let half_dt = settings.dt * 0.5;

    let is_rotating = matches!(settings.model, SimulationModel::SolidFriction(_));

    for (vel, omega, &mass, &inertia, &force, &torque) in izip!(
        &mut particles.velocity,
        &mut particles.omega,
        &particles.mass,
        &particles.inertia,
        forces,
        torques,
    ) {
        let acceleration = force / mass;       
        // Final half-step velocity update using new forces
        *vel += acceleration * half_dt;

        if is_rotating{
            let alpha=torque / inertia;
            *omega += alpha * half_dt;
        }
    }
}


/// Enforces periodic boundary conditions by wrapping a position into the primary simulation box.
///
/// This function uses a branchless floored-division approach to map any coordinate 
/// $(x, y, z)$ to the range $[0, L)$. If a particle exits the right face of the box, 
/// it "teleports" to the left face, and vice versa.
///
/// # Arguments
///
/// * `pos` - The mutable position vector to be wrapped.
/// * `sim_box_size` - The dimensions of the periodic simulation cell.
///
/// # Physics Context
///
/// The formula used is: $\mathbf{r}_{new} = \mathbf{r} - \mathbf{L} \cdot \lfloor \mathbf{r} / \mathbf{L} \rfloor$.
/// This ensures that the simulation represents an infinite tiling of the 
/// primary cell, maintaining a constant particle density.
pub fn check_periodic(pos: &mut DVec3, sim_box_size: DVec3) {
    // Branchless wrapping: more efficient than multiple if-statements for large displacements
    *pos = *pos - sim_box_size * (*pos / sim_box_size).floor();
}


/// Incrementally increases the radius of particles belonging to a specific type.
///
/// This is typically used in "compression-by-growth" protocols to reach a 
/// jammed state or to simulate swelling materials.
///
/// # Arguments
///
/// * `particles` - The mutable particle buffer.
/// * `ptype` - The specific particle category ID that should undergo growth.
///
/// # Notes
///
/// * **Mass Consistency:** Note that this only modifies the `radius` field. 
///   If your simulation physics depends on `mass`, you may need to 
///   recalculate it after calling this function to maintain a constant density.
/// * **Growth Rate:** The current multiplier is $1.00001$ ($0.001\%$) per call.
pub fn change_rad(particles: &mut ParticleVec, ptype: usize) {
    for (radius, &p) in izip!(&mut particles.radius, &particles.ptype) {
        if p == ptype {
            *radius *= 1.00001;
        }
    }
}

pub fn move_sinwave(particles: &mut ParticleVec, settings: &SimulationSettings, time: f64){
    let amplitude: f64 = 0.1;
    let frequency: f64= 250.0;

    //move surface particles up and down
    for (pos, &ptype) in izip!(&mut particles.position, &particles.ptype){
        if ptype == 1{
            let velocity_z = amplitude*(2.0*std::f64::consts::PI*frequency*time).cos();
            pos.z += velocity_z * settings.dt;
        }
    }

}

pub fn change_colour(particles: &mut ParticleVec, _settings: &SimulationSettings){
    let threshold: f64 = 0.01;
    
    let new_colour = Srgba::new(0, 255, 0, 255);
    //change colour of particles
    for (pos, col, &ptype) in izip!(&mut particles.position, &mut particles.color,  &particles.ptype){
        if (ptype == 0) && (pos.z > threshold){
                *col = new_colour; 
            }
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{create_particle_vec, create_molecule_vec, setup_single_molecule_data};
    
    #[test]
    fn test_integrate_singleparticle_update() {
        let mut particles = create_particle_vec(); // Particles at (1,2,3)
        let mut settings = SimulationSettings::default();
        settings.dt = 0.1;
        settings.sim_box_size = DVec3::new(10.0, 10.0, 10.0);

        // Force of 10.0 on Particle 0 (mass is 1.0) -> Accel = 10.0
        let mut forces = vec![DVec3::ZERO; particles.len()];
        forces[0] = DVec3::new(10.0, 0.0, 0.0);
        let mut torques = vec![DVec3::ZERO; particles.len()];
        torques[0] = DVec3::new(10.0, 0.0, 0.0);

        // Initial state: pos=1.0, vel=1.0
        // Expected Vel Half-step: 1.0 + (10.0 * 0.05) = 1.5
        // Expected Pos Full-step: 1.0 + (1.5 * 0.1) = 1.15
        integrate_singleparticle_update(&forces, &torques,  &mut particles, &settings);

        assert!((particles.velocity[0].x - 1.5).abs() < 1e-6);
        assert!((particles.position[0].x - 1.15).abs() < 1e-6);
    }

    #[test]
    fn test_integrate_singleparticle_correct() {
        let mut particles = create_particle_vec();
        let mut settings = SimulationSettings::default();
        settings.dt = 0.1;

        // Force of 10.0 in X direction. mass is 1.0, so Accel = 10.0
        let forces = vec![DVec3::new(10.0, 0.0, 0.0); particles.len()];
        let torques = vec![DVec3::new(0.0, 0.0, 0.0); particles.len()];

        // Manually set a "pre-predicted" state.
        // Let's assume the particle started at vel 1.0.
        // After the first half-kick (update), vel should be: 1.0 + (10.0 * 0.05) = 1.5
        for vel in &mut particles.velocity {
            *vel = DVec3::new(1.5, 0.0, 0.0);
        }

        // Perform the Correction (The second half-kick)
        // Mathematically: v_final = v_half + (a_new * half_dt)
        // v_final = 1.5 + (10.0 * 0.05) = 2.0
        integrate_singleparticle_correct(&forces, &torques, &mut particles, &settings);

        // Verify
        for vel in &particles.velocity {
            assert!((vel.x - 2.0).abs() < 1e-6, "Velocity correction failed to reach 2.0");
        }
    }

    #[test]
    fn test_check_periodic_wrapping() {
        let sim_box_size = DVec3::new(10.0, 10.0, 10.0);
        
        // Case 1: Just outside the right boundary (10.2 -> 0.2)
        let mut pos_right = DVec3::new(10.2, 5.0, 5.0);
        check_periodic(&mut pos_right, sim_box_size);
        assert!((pos_right.x - 0.2).abs() < 1e-6);

        // Case 2: Just outside the left boundary (-0.2 -> 9.8)
        let mut pos_left = DVec3::new(-0.2, 5.0, 5.0);
        check_periodic(&mut pos_left, sim_box_size);
        assert!((pos_left.x - 9.8).abs() < 1e-6);

        // Case 3: Multiple box lengths away (25.5 -> 5.5)
        let mut pos_far = DVec3::new(25.5, 5.0, 5.0);
        check_periodic(&mut pos_far, sim_box_size);
        assert!((pos_far.x - 5.5).abs() < 1e-6);
    }

    #[test]
    fn test_particle_growth_by_type() {
        let mut particles = create_particle_vec();
        // In your helper: Particle 0 is ptype 0, Particle 1 is ptype 1
        let original_rad_0 = particles.radius[0];
        let original_rad_1 = particles.radius[1];

        // Grow only ptype 1
        change_rad(&mut particles, 1);

        assert_eq!(particles.radius[0], original_rad_0, "Ptype 0 should not have grown");
        assert!(particles.radius[1] > original_rad_1, "Ptype 1 should have grown");
        
        // Verify the exact multiplier (1.00001)
        let expected = original_rad_1 * 1.00001;
        assert!((particles.radius[1] - expected).abs() < 1e-9);
    }

    // Checking that zero force does nothing
    #[test]
    fn test_integrate_rigid_body_conservation() {
        
        let settings = SimulationSettings { dt: 0.1, ..Default::default() };
        let mut particles = create_molecule_vec();
        let mol_data = setup_single_molecule_data(&particles); 
        
        // Calculate Initial State
        let (mass,com,initial_com_vel) = calculate_molecule_com(&vec![0, 1], &particles);
        println!("com_vel {:?}", initial_com_vel);
        
        
        let initial_omega = particles.omega[0];

        // Perform one step with zero forces
        integrate_rigid_bodies(&vec![DVec3::ZERO; 2], &vec![DVec3::ZERO; 2], &mut particles, &mol_data, &settings);
        integrate_rigid_bodies_correct(&vec![DVec3::ZERO; 2], &vec![DVec3::ZERO; 2], &mut particles, &mol_data, &settings);

        
        // Verify Conservation
        let (_,_,final_com_vel) = calculate_molecule_com(&vec![0, 1], &particles);
        

        assert!((final_com_vel - initial_com_vel).length() < 1e-12, "COM Velocity changed!");
        assert!((particles.omega[0] - initial_omega).length() < 1e-12, "Omega changed!");
    }

    #[test]
    fn test_integrate_rigid_body_gravity() {
        let dt = 0.1;
        let settings = SimulationSettings { dt, ..Default::default() };

        let mut particles = create_molecule_vec();
        let mol_data = setup_single_molecule_data(&particles);
        
        // Gravity acting only on Z
        let gravity = DVec3::new(0.0, 0.0, -9.81);
        let particle_mass = 1.0; 
        let force_vec = gravity * particle_mass;
        
        let forces = vec![force_vec; 2];
        let torques = vec![DVec3::ZERO; 2];

        // Integration
        integrate_rigid_bodies(&forces, &torques, &mut particles, &mol_data, &settings);
        integrate_rigid_bodies_correct(&forces, &torques, &mut particles, &mol_data, &settings);

        // Verify: Only Z-velocity should be affected by gravity
        // Initial velocity was (1.0, 1.0, 1.0) and (0.0, 1.0, 1.0)
        // Average X-velocity = (1.0 + 0.0) / 2 = 0.5
        // Average Z-velocity = (1.0 + 1.0) / 2 = 1.0
        let expected_z_vel = 1.0 + (gravity.z * dt);
        
        let (_, _, final_com_vel) = calculate_molecule_com(&vec![0, 1], &particles);

        assert!((final_com_vel[2] - expected_z_vel).abs() < 1e-12, "Z-axis gravity integration failed!");
        assert!((final_com_vel[0] - 0.5).abs() < 1e-12, "X-axis velocity should remain unchanged!");
    }

    #[test]
    fn test_molecule_rotation_torque() {
        let dt = 0.1;
        let settings = SimulationSettings { dt, ..Default::default() };
        let mut particles = create_molecule_vec();
        let mol_data = setup_single_molecule_data(&particles);
        let molecule=mol_data.get(&0).expect("0 should exist");

        // Apply a force couple: P0 pushed in +X, P1 pushed in -X
        // This creates rotation around the Y-axis.
        let forces = vec![DVec3::new(0.5, 0.0, 0.0), DVec3::new(-0.5, 0.0, 0.0)];
        let torques = vec![DVec3::ZERO, DVec3::ZERO];

        let init_omega = particles.omega[0];
        // Integration
        integrate_rigid_bodies(&forces, &torques, &mut particles, &mol_data, &settings);
        integrate_rigid_bodies_correct(&forces, &torques, &mut particles, &mol_data, &settings);

        let pids = &molecule.pids;
        let lead_idx = pids[0];
        let final_omega = particles.omega[0];

        let (_, _, final_com_vel) = calculate_molecule_com(&molecule.pids, &particles);
        
        println!("omega final {:?}", final_omega);
        println!("com_vel final {:?}", final_com_vel);
        // Check the omega values.
        assert!((final_omega.y - 1.071237188301297).abs() < 1e-12, "Angular velocity change failed!");
        
        // Verify COM velocity conservation
        
        assert!((final_com_vel.x - 0.5).abs() < 1e-12, "COM X-velocity should be conserved!");       
    }


}
