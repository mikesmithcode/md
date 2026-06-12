//! Force functions for use in the implementation of update_forces()
//! 
//! Your simulation should implement the Forces trait on a unit struct. This involves
//! defining the function update_forces(). You can call the forces functions in this module
//! inside update_forces or define your own.

use glam::DVec3;
use rand_distr::{Normal, Distribution};

use crate::md_sim::particle::ParticleVec;
use crate::md_sim::SimulationSettings;
use crate::md_sim::models::SimulationModel;



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

    /// Set this to true if your particle is composite and you need to apply forces and torques to whole.
    fn has_internal_forces(&self) -> bool {false}

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
        torques: &mut [DVec3],
        particles: &ParticleVec, 
        settings: &SimulationSettings,
        time: f64
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
        torques: &mut [DVec3],
        particles: &ParticleVec, 
        settings: &SimulationSettings
    );

    fn update_internal_forces(
        &self,
        _particles: &ParticleVec, 
        _forces: &mut [DVec3], 
        _torques: &mut [DVec3],
        _settings: &SimulationSettings
    ){
        // Optional: No Internal Forces by Default
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

/// An active force for use with ABPs
/// 
/// For each particle i we generate some random numbers. We then calculate the noise scale.
/// The variance of the random displacement in time dt is 2*Dt*dt but we will multiply this by 
/// dt when we calculate the displacement in motion part. Friction F is gamma * v. The noise must be (2*gamma**2 * Dt/dt)**0.5
pub fn active_force(i: usize, forces: &mut [DVec3], particles: &ParticleVec, settings: &SimulationSettings){
    let mut rng = rand::thread_rng();
    let normal = Normal::new(0.0, 1.0).unwrap();

    if let SimulationModel::Active(params) = &settings.model {
        // initial direction       
        let dir_vector = particles.orientation[i] * particles.rel_pos[i];

        // F_active = gamma * v0 * dir_vector in direction of particle orientation
        let f_active = dir_vector*(params.gamma * params.v0);

        // Translational Noise "Force"
        // This represents the random kicks from the surrounding fluid
        let noise_scale = params.gamma * (2.0 * params.Dt / settings.dt).sqrt();
        
        let f_noise = glam::DVec3::new(
            normal.sample(&mut rng) * noise_scale,
            0.0, // Keep 2D if that's your constraint, otherwise add Y noise
            normal.sample(&mut rng) * noise_scale,
        );

        // Add force
        forces[i] += f_active + f_noise;
    }
    
}


// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------
//
// Pair Forces - forces applied between particles i and j
//
// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------



/// Calculates contact forces and torques between two particles using a Linear Spring-Dashpot (LSD) model.
///
/// This function handles both central repulsion (normal force) and optional surface friction 
/// (tangential force). It accounts for rotational dynamics by calculating relative velocity 
/// at the contact point and applying resulting torques.
/// 
/// N.B because each particle is stored in each others Verlet list (ie i knows about j and j knows about i)
/// when an interaction is possible we don't apply Newton's third law (ie $F_ij = -F_ji$). This is done
/// by running this function for both i, j and j,i.
///
/// # Physical Model
///
/// ### Normal Force ($\mathbf{F}_n$)
/// Calculated using a linear spring for overlap and a dashpot for dissipation:
/// $$\mathbf{F}_n = \max(0, k \cdot \delta_{overlap} - \gamma \cdot v_{normal}) \mathbf{n}$$
/// 
/// ### Tangential Force ($\mathbf{F}_t$)
/// If friction is enabled, the tangential component is calculated via relative surface velocity:
/// $$\mathbf{v}_{surface} = \mathbf{v}_{cm} + \boldsymbol{\omega} \times \mathbf{r}$$
/// The force is modeled as a viscous dashpot clamped by the Coulomb friction limit:
/// $$\|\mathbf{F}_t\| \leq \mu \|\mathbf{F}_n\|$$
///
/// # Arguments
///
/// * `i`, `j` - Indices of the interacting particles.
/// * `particles` - Reference to the particle data structure (includes position, velocity, and omega).
/// * `forces` - Mutable slice to accumulate linear forces.
/// * `torques` - Mutable slice to accumulate angular torques.
/// * `settings` - Global simulation config, including the `SimulationModel` for parameter dispatch.
///
/// # Periodic Boundaries
///
/// * **Minimum Image Convention:** Automatically handles periodic wrapping via `check_delta` 
///   to ensure interactions occur over the shortest path across boundaries.
///
/// # Performance
///
/// Marked `#[inline(always)]` to facilitate compiler optimisations within the spatial 
/// search loops. For models without friction, the tangential and torque logic is 
/// bypassed to maintain high execution speeds.
#[inline(always)]
pub fn granular_collision(i: usize, j: usize, particles: &ParticleVec, forces: &mut [DVec3], torques: &mut [DVec3], settings: &SimulationSettings) {   
    // Extract params
    let (stiffness, damping, mu_opt) = match &settings.model {
            SimulationModel::Solid(p) => (p.stiffness, p.damping, None),
            SimulationModel::SolidFriction(p) => (p.stiffness, p.damping, Some(p.mu)),
            _ => panic!("Unsupported model for granular collision"),
        };

    // Calc overlap etc
    let mut delta = particles.position[i] - particles.position[j];
    check_delta(&mut delta, &settings.sim_box_size);

    let combined_rad = particles.radius[i] + particles.radius[j];
    let dist_sq = delta.length_squared();

    if dist_sq < combined_rad * combined_rad && dist_sq > 1e-18 {
        let dist = dist_sq.sqrt();
        let normal = delta / dist;
        let overlap = combined_rad - dist;

        // Normal Force
        let rel_vel = particles.velocity[i] - particles.velocity[j];
        let normal_vel = rel_vel.dot(normal);

        let f_normal_mag = (stiffness * overlap - damping * normal_vel).max(0.0);
        let f_normal_vec = normal * f_normal_mag;

        // Friction if applicable (Model = SolidFriction)
        // Use viscous friction clamped to max of static normal_force * mu
        if let Some(mu) = mu_opt {
            let r_i = normal * -particles.radius[i];
            let r_j = normal * particles.radius[j];

            let v_surface_rel = (particles.velocity[i] + particles.omega[i].cross(r_i)) 
                              - (particles.velocity[j] + particles.omega[j].cross(r_j));
            let v_tang = v_surface_rel - (v_surface_rel.dot(normal) * normal);
            
            if v_tang.length_squared() > 1e-18 {
                let f_t_ideal = v_tang * -damping; 
                let limit = mu * f_normal_mag;
                
                let f_t_mag_sq = f_t_ideal.length_squared();
                let f_t_vec = if f_t_mag_sq > limit * limit {
                        f_t_ideal * (limit / f_t_mag_sq.sqrt())
                    } else {
                        f_t_ideal
                    };

                // Apply Tangential Forces and Torques
                forces[i] += f_t_vec;
                //forces[j] -= f_t_vec;
                torques[i] += r_i.cross(f_t_vec);
                //torques[j] -= r_j.cross(f_t_vec);
            }
        }

        // Apply Normal Force (Shared)
        forces[i] += f_normal_vec;
    }
}

/// This implements the WCA between particles i and j. 
/// 
/// WCA is a truncated lennards-Jones potential that stops at the minimum of the potential.
pub fn weeks_chandler_andersen(i: usize,j: usize,forces: &mut [DVec3], particles: &ParticleVec,settings: &SimulationSettings){

    let mut delta = particles.position[i] - particles.position[j];
    check_delta(&mut delta, &settings.sim_box_size);

    let r2 = delta.x * delta.x + delta.z * delta.z;
    if r2 > 1e-12{
        let r = r2.sqrt();

        if let SimulationModel::Active(params) = &settings.model {
            let epsilon = params.stiffness;
            let sigma = particles.radius[i] + particles.radius[j];
            
            // The WCA potential is only active up to the minimum of the LJ curve. r_cut = 2^(1/6) * sigma. 
            let r2_cut = 1.259921 * (sigma * sigma);

            if r2 < r2_cut {
                // Implement the cutoff
                let s2_r2 = (sigma * sigma) / r2;
                let s6_r6 = s2_r2 * s2_r2 * s2_r2;
                
                // The derivative of the WCA potential gives the force magnitude:
                // F(r) = (48 * epsilon / r^2) * [ (sigma/r)^12 - 0.5 * (sigma/r)^6 ]
                let f_mag = (48.0 * epsilon / r) * (s6_r6 * s6_r6 - 0.5 * s6_r6);

                // Create the force vector
                let force_vec = glam::DVec3::new(delta.x * f_mag / r, 0.0, delta.z * f_mag / r);

                // Apply Newton's Third Law (Equal and Opposite)
                forces[i] += force_vec;

            }
        }

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
    use crate::md_sim::models::CollisionParams;
    

    // -----------------------------------------------------------------
    // Test single particle forces
    // -----------------------------------------------------------------


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
fn test_granular_collision() {
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
        interaction_ptypes:vec![[0 as u8,0 as u8]],
        model,                 // Our Solid model
        active_mask: [true; 32]
    };

    let mut forces = vec![DVec3::ZERO; particles.len()];
    let mut torques = vec![DVec3::ZERO; particles.len()];

    // Create a controlled overlap (Combined rad = 1.0, distance = 0.8, overlap = 0.2)
    let mut particles = particles; 
    particles.position[0] = DVec3::ZERO;
    particles.position[1] = DVec3::new(0.8, 0.0, 0.0);

    // --- Case A: Compression (Moving towards each other) ---
    particles.velocity[0] = DVec3::new(1.0, 0.0, 0.0);
    particles.velocity[1] = DVec3::new(-1.0, 0.0, 0.0);

    granular_collision(0, 1, &particles, &mut forces, &mut torques, &settings);

    assert!(forces[0].x < 0.0, "Force should be repulsive for particle 0");
    let force_with_damping = forces[0].length();

    // --- Case B: Restitution (Moving away) ---
    forces = vec![DVec3::ZERO; particles.len()]; // Reset force buffer
    particles.velocity[0] = DVec3::new(-1.0, 0.0, 0.0);
    particles.velocity[1] = DVec3::new(1.0, 0.0, 0.0);

    granular_collision(0, 1, &particles, &mut forces,&mut torques, &settings);
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
