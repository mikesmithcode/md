
//-------------------------------------------------------------------------------------------------------
// Integration functions
//-------------------------------------------------------------------------------------------------------


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


//Not yet tested
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
