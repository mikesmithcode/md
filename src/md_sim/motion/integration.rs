//-------------------------------------------------------------------------------------------------------
// Integration functions
//-------------------------------------------------------------------------------------------------------

use glam::{DVec3, DMat3, DQuat};
use itertools::izip;
use std::collections::HashMap;

use crate::md_sim::motion::change::enforce_boundary;
use crate::md_sim::utils::check_delta;
use crate::md_sim::{SimulationSettings, ParticleVec};
use crate::md_sim::particle::{SimulationModel, calculate_molecule_com, MoleculeData};


//-------------------------------------------------------------------------------------------------------
// No torques or rotations etc
//-------------------------------------------------------------------------------------------------------



/// Performs the first half of the Velocity Verlet integration for standard point particles (Prediction). 
/// Must be used together with `integrate_singleparticle_correct`.
///
/// # Arguments
/// 
/// * `forces` - Slice of force vectors acting on each particle from the previous timestep. One `DVec3` per particle.
/// * `particles` - Mutable reference to the particle buffers containing positions, velocities, masses, etc.
/// * `settings` - Simulation settings providing timestep (`dt`), box dimensions, and boundary conditions.
/// 
/// # Notes
/// 
/// N.B. This function is strictly for point particles not subject to internal torques, orientations, or rotational degrees of freedom.
/// 
/// This function should be called inside `update_motion`. It uses the forces from the **previous** timestep to:
/// 
/// 1. Update velocities by a half-step: 
///    $v(t + \frac{\Delta t}{2}) = v(t) + \frac{a(t)\Delta t}{2}$
/// 2. Update positions by a full step: 
///    $x(t + \Delta t) = x(t) + v(t + \frac{\Delta t}{2})\Delta t$
/// 3. Enforce boundary conditions on the newly updated positions and velocities.
///
/// After this call, positions are finalized for the current step, allowing 
/// for new force calculations (e.g., collisions) at $x(t + \Delta t)$.
pub fn integrate_singleparticle_update(
    forces: &[DVec3], 
    particles: &mut ParticleVec, 
    settings: &SimulationSettings
) {
    let dt = settings.dt;
    let half_dt = dt * 0.5;
    let sim_box_size = settings.sim_box_size;
    let periodic = settings.periodic;

    let _is_rotating = matches!(settings.model, SimulationModel::SolidFriction(_));

    for (pos, vel, &mass, &force) in izip!(
        &mut particles.position,
        &mut particles.velocity,
        &particles.mass,
        forces, 
    ) {
        let acceleration = force / mass;
        
        // Half-step velocity update
        *vel += acceleration * half_dt;
        // Full-step position update
        *pos += *vel * dt;
        
        // Apply boundary conditions
        enforce_boundary(pos, vel, sim_box_size, periodic);
    }
}

/// Performs the second half of the Velocity Verlet integration for standard point particles (Correction).
/// 
/// # Arguments
/// 
/// * `forces` - Slice of force vectors acting on each particle, calculated at the **new** positions ($t + \Delta t$). One `DVec3` per particle.
/// * `particles` - Mutable reference to the particle buffers containing positions, velocities, masses, etc.
/// * `settings` - Simulation settings providing timestep (`dt`) and other configuration parameters.
/// 
/// # Notes
/// 
/// N.B. This function is strictly for standard particles not subject to internal torques, orientations, or rotational degrees of freedom.
/// Must be used in tandem with `integrate_singleparticle_update`.
///
/// This function should be called inside `correct_motion`. It completes the Velocity Verlet cycle for point particles by 
/// using the forces calculated at the **new** positions to finalize their velocities:
/// $v(t + \Delta t) = v(t + \frac{\Delta t}{2}) + \frac{a(t + \Delta t)\Delta t}{2}$
pub fn integrate_singleparticle_correct(
    forces: &[DVec3], 
    particles: &mut ParticleVec, 
    settings: &SimulationSettings
) {
    let half_dt = settings.dt * 0.5;

    for (vel, &mass,&force) in izip!(
        &mut particles.velocity,
        &particles.mass,
        forces
    ) {
        let acceleration = force / mass;       
        // Final half-step velocity update using new forces
        *vel += acceleration * half_dt;
    }
}


//------------------------------------------------------------------------------------------------------
// Rotations included
//------------------------------------------------------------------------------------------------------

/// Performs the prediction step of the Velocity Verlet integration for multiparticle rigid bodies, 
/// incorporating both translational and rotational dynamics.
/// 
/// # Arguments
/// 
/// * `forces` - Slice of force vectors acting on each particle, calculated in the previous timestep.
/// * `torques` - Slice of torque vectors acting on each particle, calculated in the previous timestep.
/// * `particles` - Mutable reference to the particle buffers containing positions, velocities, orientations, etc.
/// * `molecule_map` - Reference mapping molecule IDs to their constituent particle IDs and internal rigid-body properties.
/// * `settings` - Simulation settings providing timestep (`dt`), box dimensions, and boundary conditions.
///
/// # Notes
/// 
/// This function should be called inside `update_motion`. For each rigid molecule, it treats the 
/// collection of particles as a single rigid body by aggregating forces and torques at the Center of Mass (COM):
/// 
/// 1. **Translational & Rotational Kinetics:** Sums forces to update the COM linear velocity and aggregates 
///    torques (including internal offset torques) to update angular velocity via Euler's rotation equations 
///    with gyroscopic terms.
/// 2. **Orientation & Position Integration:** Advances the molecule's COM position and integrates its 
///    orientation quaternion using the updated angular velocity and a scaled axis delta rotation.
/// 3. **Collective Particle Update:** Rather than integrating each particle independently, all particles within 
///    a single molecule are updated **cohesively**. Their individual positions, velocities, orientations, and 
///    angular velocities are re-derived simultaneously from the molecule's new COM state and rotated relative positions 
///    ($r_{\text{global}} = R \cdot r_{\text{local}}$), ensuring internal rigid-body constraints remain perfectly locked.
/// 
/// N.B. If boundary conditions are non-periodic, boundary enforcement on individual particles will reflect 
/// velocities but will not alter global molecule-level angular momentum or center of mass trajectory automatically.
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
    let periodic = settings.periodic;

    for (mol_id, mol) in molecule_map {   
        let lead_idx = mol.pids[0];    
        
        // Calculate current COM etc
        let (total_mass, com_pos, com_vel) = calculate_molecule_com(&mol.pids, &particles);
    
        // Calculate aggregate forces and torques
        let mut total_force = DVec3::ZERO;
        let mut total_torque = DVec3::ZERO;
        for &idx in &mol.pids {
            total_force += forces[idx];
            let mut delta_r = particles.position[idx] - com_pos;
            check_delta(&mut delta_r, sim_box_size, periodic);
            total_torque += torques[idx] + delta_r.cross(forces[idx]);
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

            enforce_boundary(&mut particles.position[idx], &mut particles.velocity[idx], settings.sim_box_size, settings.periodic);
        }
    }
}

/// Performs the second half of the Velocity Verlet integration for multiparticle rigid bodies (Correction),
/// finalizing velocities and angular velocities using forces calculated at the new positions.
/// 
/// # Arguments
/// 
/// * `forces` - Slice of force vectors acting on each particle, calculated at the **new** positions ($t + \Delta t$).
/// * `torques` - Slice of torque vectors acting on each particle, calculated at the **new** positions ($t + \Delta t$).
/// * `particles` - Mutable reference to particle buffers containing updated positions, intermediate velocities, etc.
/// * `molecule_map` - Reference mapping molecule IDs to their constituent particle IDs and internal rigid-body properties.
/// * `settings` - Simulation settings providing timestep (`dt`), box dimensions, and boundary conditions.
///
/// # Notes
/// 
/// This function should be called inside `correct_motion`. It completes the Velocity Verlet cycle for each rigid molecule:
/// 
/// 1. **Force & Torque Re-evaluation:** Aggregates the newly evaluated forces and torques across all constituent 
///    particles relative to the molecule's Center of Mass (COM).
/// 2. **Velocity & Angular Velocity Finalization:** 
///    * Updates linear COM velocity by adding the new acceleration scaled by a half-step: 
///      $v(t + \Delta t) = v(t + \frac{\Delta t}{2}) + \frac{a(t + \Delta t)\Delta t}{2}$
///    * Updates angular velocity using Euler's rotational equations with updated global inertia tensors and gyroscopic terms.
/// 3. **Cohesive Particle State Sync:** Re-distributes the finalized COM linear velocity and angular velocity 
///    across all particles in the molecule simultaneously. Individual particle velocities are updated via 
///    $v_i = v_{\text{com}} + (\omega \times r_{\text{global}})$, ensuring consistency across the rigid structure.
pub fn integrate_rigid_bodies_correct(
    forces: &[DVec3], 
    torques: &[DVec3],
    particles: &mut ParticleVec, 
    molecule_map: &HashMap<usize, MoleculeData>,
    settings: &SimulationSettings
) {
    let half_dt = settings.dt * 0.5;
    let sim_box_size = settings.sim_box_size;
    let periodic = settings.periodic;

    for (_m_id, mol) in molecule_map {
        let lead_idx = mol.pids[0];
        
        // Calculate new Force/Torque at the new position
        let (total_mass, com_pos, com_vel) = calculate_molecule_com(&mol.pids, particles);
        let mut total_force = DVec3::ZERO;
        let mut total_torque = DVec3::ZERO;
        for &idx in &mol.pids {
            total_force += forces[idx];
            let mut delta_r = particles.position[idx] - com_pos;
            check_delta(&mut delta_r, sim_box_size, periodic);
            total_torque += torques[idx] + delta_r.cross(forces[idx]);
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


//Not yet tested
pub fn update_abps(forces: &[DVec3], particles: &mut ParticleVec, settings: &SimulationSettings) {

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
            
            // Apply boundary conditions
            enforce_boundary(&mut particles.position[i], &mut particles.velocity[i], settings.sim_box_size, settings.periodic);
        }
    }
}
