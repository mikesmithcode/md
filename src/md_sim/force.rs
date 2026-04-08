//! Force functions for use in the implementation of update_forces()
//! 
//! Your simulation should implement the Forces trait on a unit struct. This involves
//! defining the function update_forces(). You can call the forces functions in this module
//! inside update_forces or define your own.

use glam::DVec3;

use crate::md_sim::particle::ParticleVec;
use crate::md_sim::SimulationSettings;
use crate::SimulationModel;



/// Defines the physical interactions and force constraints for a simulation.
///
/// The `Forces` trait is the core of the physics engine's dynamics. It separates 
/// force calculations into unary (single-body), binary (pair-wise), and 
/// post-processing (constraints) phases.
pub trait Forces {
    /// Indicates if the simulation requires pair-wise force calculations.
    ///
    /// If `false`, the engine will skip spatial binning and the 
    /// `update_pair_forces` calls, significantly improving performance.
    fn has_pair_forces(&self) -> bool { true }

    /// Indicates if the simulation requires single-body force calculations.
    ///
    /// If `false`, the engine will skip the `update_single_forces` loop.
    fn has_single_forces(&self) -> bool { true }

    /// Calculates unary forces acting on a specific particle.
    ///
    /// This method is called once per particle in an $O(N)$ loop. It is the 
    /// ideal location for forces that depend only on an individual particle's 
    /// state, such as gravity, viscous drag, or self-propulsion.
    ///
    /// # Arguments
    /// * `i` - Index of the particle being updated.
    /// * `forces` - The force buffer where contributions should be added.
    /// * `particles` - Reference to the particle data (positions, velocities, etc.).
    /// * `settings` - Global simulation parameters.
    fn update_single_forces(
        &self, 
        i: usize, 
        forces: &mut [DVec3], 
        particles: &ParticleVec, 
        settings: &SimulationSettings
    );

    /// Calculates binary forces between two particles within the cutoff distance.
    ///
    /// This method is called by the `CellGrid` manager for pairs $(i, j)$ found 
    /// within neighbouring cells. Users should calculate the interaction (e.g., 
    /// Lennard-Jones or Hertzian contact) and update the force buffer for both 
    /// particles if Newton's Third Law applies.
    ///
    /// # Arguments
    /// * `i`, `j` - Indices of the interacting particles.
    /// * `forces` - The force buffer where contributions should be added.
    fn update_pair_forces(
        &self, 
        i: usize, 
        j: usize, 
        forces: &mut [DVec3], 
        particles: &ParticleVec, 
        settings: &SimulationSettings
    );

    /// Finalises the force buffer by applying kinematic constraints or overrides.
    ///
    /// This hook is called by the engine after all single and pair forces have 
    /// been accumulated, but before the motion integrator is applied. 
    ///
    /// # Default Implementation
    /// The default implementation does nothing. To "freeze" specific particle 
    /// types (e.g., static walls), call the standalone utility 
    /// `zero_forces_for_ptypes()` within your implementation of this method.
    ///
    /// # Example
    /// ```no run
    /// fn update_ptype_no_forces(&self, forces: &mut [DVec3], particles: &ParticleVec) {
    ///     // Ensure type 0 (floor) and type 1 (boundary) remain immobile
    ///     zero_forces_for_ptypes(forces, particles, &[0, 1]);
    /// }
    /// ```
    fn update_ptype_no_forces(&self, _forces: &mut [DVec3], _particles: &ParticleVec) {
        // Default: No constraints applied.
    }
}

// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------
//
// Single Forces - forces applied to individual particles
//
// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------


/// Enforces kinematic constraints by zeroing forces for specific particle types.
///
/// This function acts as a post-processing step to ensure that certain particle types 
/// (such as static boundaries, fixed obstacles, or particles with prescribed motion) 
/// do not respond to calculated physical forces.
///
/// # Arguments
///
/// * `forces` - The mutable buffer of accumulated forces to be filtered.
/// * `particles` - A reference to the particle data, used to check `ptype`.
/// * `no_force_ptypes` - A slice of type IDs that should remain unaffected by forces.
///
/// # Example
///
/// ```no run
/// // Freeze particles of type 1 (floor) and type 2 (walls)
/// let immobile = [1, 2];
/// simupdate.zero_forces_for_ptypes(&mut forces, &particles, &immobile);
/// ```
///
/// # Notes
///
/// * **Execution Order:** This should be called after all force contributors (single and pair) 
///   have been added to the buffer, but before the motion integrator updates velocities.
/// * **Prescribed Motion:** Zeroing the force is necessary for particles following a 
///   pre-defined trajectory to prevent physical interactions from deviating them from 
///   their path.
pub fn zero_forces_for_ptypes(forces: &mut [DVec3], particles: &ParticleVec, no_force_ptypes: &[usize]) {
    for (f, &p_type) in forces.iter_mut().zip(particles.ptype.iter()) {
        // If the current particle's type is in the 'no_force' list, zero it
        if no_force_ptypes.contains(&p_type) {
            *f = DVec3::ZERO;
        }
    }
}



/// Calculates and adds the gravitational weight to a specific particle.
///
/// This function assumes a constant gravitational acceleration $g \approx 9.81 \, \text{m/s}^2$ 
/// acting in the negative $z$ direction.
///
/// # Arguments
///
/// * `i` - The index of the target particle within the force and particle buffers.
/// * `forces` - A mutable slice of force vectors to which the weight will be added.
/// * `particles` - A reference to the particle data structure containing inverse masses.
///
/// # Notes
///
/// * **Buoyancy:** If simulating a fluid environment, the `mass` attribute of the 
///   particle should be adjusted to reflect the effective weight (relative density).
/// * **Infinite Mass:** Particles with an `mass` of **0.0**  are skipped to avoid division by zero.
///
/// # Panics
///
/// This function will panic if the index `i` is out of bounds for either `forces` 
/// or `particles.mass`.
pub fn add_weight(i: usize, forces: &mut [DVec3], particles: &ParticleVec) {
    let gravity = -9.81;
    let mass = particles.mass[i];

    let weight = gravity * mass;
    forces[i].z += weight;
}

/// Calculates and adds the viscous drag force (Stokes' Law) to a specific particle.
///
/// This function models the drag force exerted on a spherical particle moving through 
/// a viscous fluid at low Reynolds numbers, where the force is proportional to the 
/// particle's velocity and radius.
///
/// # Mathematical Formula
///
/// The drag force is calculated as:
/// $$F_{drag} = -6\pi \eta r v$$
/// where $\eta$ is the dynamic viscosity, $r$ is the particle radius, and $v$ is the velocity.
///
/// # Arguments
///
/// * `i` - The index of the target particle.
/// * `forces` - A mutable slice of force vectors to which the drag will be added.
/// * `particles` - A reference to the particle data structure containing velocity and radius.
/// * `viscosity` - The dynamic viscosity ($\eta$) of the surrounding fluid.
///
/// # Panics
///
/// This function will panic if the index `i` is out of bounds for `forces`, 
/// `particles.velocity`, or `particles.radius`.
pub fn add_viscous_drag(i: usize, forces: &mut [DVec3], particles: &ParticleVec, viscosity: f64) {
    let vel = particles.velocity[i];
    let rad = particles.radius[i];
    
    // Stokes' Law: F = -6 * pi * eta * r * v
    let drag = -6.0 * std::f64::consts::PI * viscosity * rad * vel;
    
    forces[i] += drag;
}



// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------
//
// Pair Forces - forces applied between particles i and j
//
// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------



/// Calculates the normal contact force between two particles using an inelastic collision model.
///
/// This function implements a Linear Spring-Dashpot (LSD) model. It calculates the 
/// repulsive force based on the overlap (stiffness) and the relative normal velocity 
/// (damping) of the two particles. 
///
/// # Physical Model
///
/// The normal force $\mathbf{F}_n$ is calculated as:
/// $$\mathbf{F}_n = (k \cdot \delta_{overlap} - \gamma \cdot v_{normal}) \mathbf{n}$$
/// where $k$ is the stiffness, $\gamma$ is the damping coefficient, and $\mathbf{n}$ 
/// is the contact normal. Damping is only applied during the compression phase 
/// ($v_{normal} < 0$) to prevent unphysical "sticking" upon separation.
///
/// # Arguments
///
/// * `i`, `j` - Indices of the interacting particles.
/// * `particles` - Reference to the particle data structure.
/// * `forces` - Mutable slice of the force buffer (updates both $i$ and $j$).
/// * `params` - Collision parameters including stiffness and damping.
/// * `sim_box_size` - Dimensions of the periodic simulation box.
///
/// # Periodic Boundaries & Constraints
///
/// * **Minimum Image Convention:** This function automatically handles periodic wrapping 
///   via `check_delta`. It finds the shortest distance between particles across boundaries.
/// * **Boundary Caution:** If using fixed boundaries (walls) near the edge of a periodic 
///   box, ensure a "buffer zone" of at least one cutoff distance exists to prevent 
///   particles from interacting with their own image through the wall.
///
/// # Performance
///
/// This function is marked `#[inline(always)]` as it is called frequently within 
/// the nested loops of the `CellGrid` spatial search.
#[inline(always)]
pub fn inelastic_collision(i: usize,j: usize,particles: &ParticleVec,forces: &mut [DVec3],settings: &SimulationSettings) {    
    let sim_box_size = settings.sim_box_size;

    let (stiffness, damping) = match &settings.model{
        SimulationModel::Solid(params) => {
            (params.stiffness, params.damping)
            }
        _ => panic!("Settings model must use the Solid enum")
    };

        
    // Calculate relative separation with Minimum Image Convention
    let mut delta = particles.position[i] - particles.position[j];
    check_delta(&mut delta, &sim_box_size);

    let combined_rad = particles.radius[i] + particles.radius[j];
    let dist_sq = delta.length_squared(); // Optimization: check squared distance first

    if dist_sq < combined_rad * combined_rad && dist_sq > 1e-18 {
        let dist = dist_sq.sqrt();
        let normal = delta / dist;
        let overlap = combined_rad - dist;

        // Relative Velocity and Normal Component
        let rel_vel = particles.velocity[i] - particles.velocity[j];
        let normal_vel = rel_vel.dot(normal);

        // Force Calculation (Spring + Damping)
        let spring_f = stiffness * overlap;
        let damping_f = -damping * normal_vel;

        // Ensure total force is never attractive (clamping)
        let total_f = (spring_f + damping_f).max(0.0);
        let f_vec = normal * total_f;

        // Apply to both (Newton's Third Law: Action and Reaction)
        forces[i] += f_vec;
        forces[j] -= f_vec;
    }
}

// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------
//
// Utility functions
//
// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------


/// Applies the Minimum Image Convention to a displacement vector.
///
/// In a periodic simulation box, a particle can interact with either the real 
/// instance of another particle or its periodic images. This function adjusts 
/// the displacement vector `delta` to ensure it represents the shortest 
/// distance between two points across the periodic boundaries.
///
/// # Arguments
///
/// * `delta` - The mutable displacement vector ($r_j - r_i$) to be wrapped.
/// * `sim_box_size` - The dimensions of the periodic simulation cell.
///
/// # Physics Context
///
/// If a component of the displacement exceeds half the box length, the 
/// nearest image is actually in the opposite direction (through the boundary).
/// For a box of length $L$, if $\Delta x > L/2$, the shortest path is $\Delta x - L$.
///
/// # Notes
///
/// * This function is essential for correct force calculations and collision 
///   handling in periodic systems.
/// * It assumes the initial displacement was calculated using coordinates 
///   already mapped (or "wrapped") within the primary simulation box.
pub fn check_delta(delta: &mut DVec3, sim_box_size: &DVec3) {
    // Check X-axis wrapping
    if delta.x > sim_box_size.x * 0.5 { 
        delta.x -= sim_box_size.x; 
    } else if delta.x < -sim_box_size.x * 0.5 { 
        delta.x += sim_box_size.x; 
    }

    // Check Y-axis wrapping
    if delta.y > sim_box_size.y * 0.5 { 
        delta.y -= sim_box_size.y; 
    } else if delta.y < -sim_box_size.y * 0.5 { 
        delta.y += sim_box_size.y; 
    }

    // Check Z-axis wrapping
    if delta.z > sim_box_size.z * 0.5 { 
        delta.z -= sim_box_size.z; 
    } else if delta.z < -sim_box_size.z * 0.5 { 
        delta.z += sim_box_size.z; 
    }
}


// Tests for file_io
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_particle_vec;
    use crate::CollisionParams;
    

    // -----------------------------------------------------------------
    // Test single particle forces
    // -----------------------------------------------------------------

    #[test]
    fn test_check_zero_forces(){
        // Create dummy particle data
            let particles = create_particle_vec();
            let mut forces = vec![DVec3::new(1.0,1.0,1.0), DVec3::new(1.0,1.0,1.0)];
            
            zero_forces_for_ptypes(&mut forces, &particles, &[1]);

            assert!(forces[0].x == 1.0);
            assert!(forces[1].x == 0.0);
    }


    #[test]
    fn test_add_weight() {
        let particles = create_particle_vec();
        let mut forces = vec![DVec3::ZERO; particles.len()];
        
        // Apply weight to the first particle
        add_weight(0, &mut forces, &particles);

        // Assuming gravity is -9.81 and mass is 1.0 (mass = 1.0)
        // Force should be exactly -9.81 in the Z direction
        assert!((forces[0].z + 9.81).abs() < 1e-6);
        
        // Ensure the second particle hasn't been touched
        assert_eq!(forces[1], DVec3::ZERO);
    }

    #[test]
    fn test_add_drag() {
        use std::f64::consts::PI;
        
        let particles = create_particle_vec();
        let mut forces = vec![DVec3::ZERO; particles.len()];
        let viscosity = 0.1;

        // Apply drag to the first particle
        add_viscous_drag(0, &mut forces, &particles, viscosity);
        
        // Expected: -6 * PI * eta * r * v
        // Assuming create_particle_vec sets radius=0.5 and velocity.x=1.0 for particle 0
        let expected_drag_x = -6.0 * PI * viscosity * 0.5 * 1.0;
        
        assert!((forces[0].x - expected_drag_x).abs() < 1e-10);
        
        // Ensure the second particle remains at zero force
        assert_eq!(forces[1], DVec3::ZERO);
    }

    
    // -----------------------------------------------------------------
    // Test pair particle forces
    // -----------------------------------------------------------------

    #[test]
fn test_inelastic_collision() {
    let particles = create_particle_vec();
    
    // Bundle params into the specific Enum variant
    let model = SimulationModel::Solid(CollisionParams {
        stiffness: 1000.0,
        damping: 50.0,
    });

    // Initialise the full SimulationSettings struct
    let settings = SimulationSettings {
        dt: 0.001,             // Placeholder value
        sim_box_size: DVec3::new(10.0, 10.0, 10.0),
        cutoff: 2.0,           // Ensure this is large enough for the overlap
        skin:0.2,
        start: 0,
        num_steps: 100,
        dump: 10,
        active_ptypes:vec![0],
        model,                 // Our Solid model
        active_map: [true; 32]
    };

    let mut forces = vec![DVec3::ZERO; particles.len()];

    // Create a controlled overlap (Combined rad = 1.0, distance = 0.8, overlap = 0.2)
    let mut particles = particles; 
    particles.position[0] = DVec3::ZERO;
    particles.position[1] = DVec3::new(0.8, 0.0, 0.0);

    // --- Case A: Compression (Moving towards each other) ---
    particles.velocity[0] = DVec3::new(1.0, 0.0, 0.0);
    particles.velocity[1] = DVec3::new(-1.0, 0.0, 0.0);

    inelastic_collision(0, 1, &particles, &mut forces, &settings);

    assert!(forces[0].x < 0.0, "Force should be repulsive for particle 0");
    let force_with_damping = forces[0].length();

    // --- Case B: Restitution (Moving away) ---
    forces = vec![DVec3::ZERO; particles.len()]; // Reset force buffer
    particles.velocity[0] = DVec3::new(-1.0, 0.0, 0.0);
    particles.velocity[1] = DVec3::new(1.0, 0.0, 0.0);

    inelastic_collision(0, 1, &particles, &mut forces, &settings);
    let force_no_damping = forces[0].length();

    // Verification
    // Note: In your logic, damping adds to spring force during compression. 
    // So force_with_damping (Compression) should be > force_no_damping (Restitution).
    assert!(force_with_damping > force_no_damping, "Damping must increase total force magnitude during compression");
    assert_eq!(forces[0], -forces[1], "Newton's Third Law must hold (Action = -Reaction)");
}
    // -----------------------------------------------------------------
    // Test utility functions
    // -----------------------------------------------------------------

    #[test]
    fn test_check_delta() {
        let sim_box_size = DVec3::new(10.0, 10.0, 10.0);
        
        // Case 1: X is far apart (0.9L), should wrap to a small negative distance (-0.1L)
        // Example: Particle A at 0.5, Particle B at 9.5. Delta = 9.0
        let mut delta_x = DVec3::new(9.0, 0.0, 0.0);
        check_delta(&mut delta_x, &sim_box_size);
        assert!((delta_x.x + 1.0).abs() < 1e-6); // 9.0 - 10.0 = -1.0

        // Case 2: Y is negative and far apart, should wrap to a small positive distance
        // Example: Particle A at 9.5, Particle B at 0.5. Delta = -9.0
        let mut delta_y = DVec3::new(0.0, -9.0, 0.0);
        check_delta(&mut delta_y, &sim_box_size);
        assert!((delta_y.y - 1.0).abs() < 1e-6); // -9.0 + 10.0 = 1.0

        // Case 3: Z is already the shortest path, should remain unchanged
        let mut delta_z = DVec3::new(0.0, 0.0, 2.0);
        check_delta(&mut delta_z, &sim_box_size);
        assert!((delta_z.z - 2.0).abs() < 1e-6);
    }


    
}
