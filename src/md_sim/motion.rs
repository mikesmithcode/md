//! motion of the particles in the broadest sense
//! 
//! Each simulation should define a Motion trait which requires you to implement
//! update_motion() which occurs before forces are calculated and correct_motion() which occurs afterwards.

use crate::{SimulationSettings, md_sim::particle::ParticleVec};
use glam::DVec3;
use itertools::izip;

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
        forces: &[DVec3], 
        particles: &mut ParticleVec, 
        settings: &SimulationSettings
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
        _particles: &mut ParticleVec, 
        _settings: &SimulationSettings
    ) {
        // Optional: No correction by default
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
pub fn integrate_verlet_update(
    forces: &[DVec3], 
    particles: &mut ParticleVec, 
    settings: &SimulationSettings
) {
    let dt = settings.dt;
    let half_dt = dt * 0.5;
    let sim_box_size = settings.sim_box_size;

    for (pos, vel, &inv_mass, &force) in izip!(
        &mut particles.position,
        &mut particles.velocity,
        &particles.inv_mass,
        forces
    ) {
        let acceleration = force * inv_mass;
        
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
pub fn integrate_verlet_correct(
    forces: &[DVec3], 
    particles: &mut ParticleVec, 
    settings: &SimulationSettings
) {
    let half_dt = settings.dt * 0.5;

    for (vel, &inv_mass, &force) in izip!(
        &mut particles.velocity,
        &particles.inv_mass,
        forces
    ) {
        let acceleration = force * inv_mass;
        
        // Final half-step velocity update using new forces
        *vel += acceleration * half_dt;
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
///   If your simulation physics depends on `inv_mass`, you may need to 
///   recalculate it after calling this function to maintain a constant density.
/// * **Growth Rate:** The current multiplier is $1.00001$ ($0.001\%$) per call.
pub fn change_rad(particles: &mut ParticleVec, ptype: usize) {
    for (radius, &p) in izip!(&mut particles.radius, &particles.ptype) {
        if p == ptype {
            *radius *= 1.00001;
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_particle_vec;

    #[test]
    fn test_integrate_verlet_update() {
        let mut particles = create_particle_vec(); // Particles at (1,2,3)
        let mut settings = SimulationSettings::default();
        settings.dt = 0.1;
        settings.sim_box_size = DVec3::new(10.0, 10.0, 10.0);

        // Force of 10.0 on Particle 0 (inv_mass is 1.0) -> Accel = 10.0
        let mut forces = vec![DVec3::ZERO; particles.len()];
        forces[0] = DVec3::new(10.0, 0.0, 0.0);

        // Initial state: pos=1.0, vel=1.0
        // Expected Vel Half-step: 1.0 + (10.0 * 0.05) = 1.5
        // Expected Pos Full-step: 1.0 + (1.5 * 0.1) = 1.15
        integrate_verlet_update(&forces, &mut particles, &settings);

        assert!((particles.velocity[0].x - 1.5).abs() < 1e-6);
        assert!((particles.position[0].x - 1.15).abs() < 1e-6);
    }

    #[test]
    fn test_integrate_verlet_correct() {
        let mut particles = create_particle_vec();
        let mut settings = SimulationSettings::default();
        settings.dt = 0.1;

        // Force of 10.0 in X direction. inv_mass is 1.0, so Accel = 10.0
        let forces = vec![DVec3::new(10.0, 0.0, 0.0); particles.len()];

        // Manually set a "pre-predicted" state.
        // Let's assume the particle started at vel 1.0.
        // After the first half-kick (update), vel should be: 1.0 + (10.0 * 0.05) = 1.5
        for vel in &mut particles.velocity {
            *vel = DVec3::new(1.5, 0.0, 0.0);
        }

        // Perform the Correction (The second half-kick)
        // Mathematically: v_final = v_half + (a_new * half_dt)
        // v_final = 1.5 + (10.0 * 0.05) = 2.0
        integrate_verlet_correct(&forces, &mut particles, &settings);

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

    



}
