// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------
//
// Pair Forces - forces applied between particles i and j
//
// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------

use glam::DVec3;
use std::f64::consts::PI;

use crate::md_sim::SimulationSettings;
use crate::md_sim::particle::{ParticleVec, SimulationModel};
use crate::md_sim::utils::check_delta;


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
///   to ensure interactions occur over the shortest path across boundaries. check_delta handles
///   whether a boundary is periodic or not and changes behaviour accordingly.
///
/// # Performance
///
/// Marked `#[inline(always)]` to facilitate compiler optimisations within the spatial 
/// search loops. For models without friction, the tangential and torque logic is 
/// bypassed to maintain high execution speeds.
#[inline(always)]
pub fn add_particle_particle_collision(
    i: usize, 
    j: usize, 
    particles: &ParticleVec, 
    mut force: DVec3, 
    mut torque: DVec3, 
    settings: &SimulationSettings
) -> (DVec3, DVec3) { 
    
    // Rule 1: If particle i is not a collision ptype then ignore this fn.
    let is_i_coll = settings.collision_ptypes.contains(&(particles.ptype[i] as u8));
    if !is_i_coll {
        return (force, torque);
    }

  

    // Extract params
    let (stiffness, damping, mu) = match &settings.model {
        SimulationModel::Frictional(p) => (p.stiffness, p.damping, p.mu),
        _ => panic!("Unsupported model for granular collision"),
    };

    // Calc separation etc
    let mut delta = particles.position[i] - particles.position[j];
    check_delta(&mut delta, settings.sim_box_size, settings.periodic);

    let combined_rad = particles.radius[i] + particles.radius[j];
    let dist_sq = delta.length_squared();

    // contact?
    if dist_sq < combined_rad * combined_rad {
        let dist = dist_sq.sqrt();
        let normal = delta / dist; 
        let overlap = combined_rad - dist; 

        // --- EFFECTIVE MASS CORRECTION ---
        // Check if particle j is a collision ptype
        let is_j_coll = settings.collision_ptypes.contains(&(particles.ptype[j] as u8));
       // -----------------------------
        let m_i = particles.mass[i];
        
        // Rule 2 & 3: If j is a collision ptype, use its real mass. 
        // If j is NOT a collision ptype, treat it as infinite mass.
        let m_eff = if !is_j_coll {
            m_i
        } else {
            let m_j = particles.mass[j];
            (m_i * m_j) / (m_i + m_j)
        };

        let mass_scale = m_eff / m_i;
        let eff_stiffness = stiffness * mass_scale;
        let eff_damping = damping * mass_scale;
        // ---------------------------------

        // Normal Force
        let rel_vel = particles.velocity[i] - particles.velocity[j];
        let normal_vel = rel_vel.dot(normal);

        let f_normal_mag = (eff_stiffness * overlap - eff_damping * normal_vel).max(0.0);
        let f_normal_vec = normal * f_normal_mag;

        let r_i = normal * (-particles.radius[i] + overlap / 2.0);
        let r_j = normal * (particles.radius[j] - overlap / 2.0);

        let v_surface_rel = (particles.velocity[i] + particles.omega[i].cross(r_i)) 
                            - (particles.velocity[j] + particles.omega[j].cross(r_j));
        let v_tang = v_surface_rel - (v_surface_rel.dot(normal) * normal);
        
        if v_tang.length_squared() > 1e-18 {
            let f_t_ideal = v_tang * -eff_damping; 
            let limit = mu * f_normal_mag;
            
            let f_t_mag_sq = f_t_ideal.length_squared();
            let f_t_vec = if f_t_mag_sq > limit * limit {
                f_t_ideal * (limit / f_t_mag_sq.sqrt())
            } else {
                f_t_ideal
            };

            // Apply Tangential Forces and Torques
            force += f_t_vec;
            torque += r_i.cross(f_t_vec);
        }
    
        // Apply Normal Force
        force += f_normal_vec;
    }

    (force, torque)
}



/// Computes the electrostatic Coulomb force between two charged particles.
///
/// Applies the electrostatic interaction according to Coulomb's Law:
/// $$F = \frac{1}{4\pi\varepsilon_0} \frac{q_i q_j}{r^2} \hat{r}$$
///
/// # Notes
///
/// * **Asymmetric Application:** This function computes and applies the force acting on particle `i`. 
///   The reciprocal force on particle `j` is naturally handled when the pair $(j, i)$ is processed 
///   if explicitly included in the simulation's `interaction_ptypes` configuration.
///
/// # Arguments
///
/// * `i` - Index of the primary particle receiving the force.
/// * `j` - Index of the interacting neighbor particle.
/// * `particles` - Reference to particle state buffers containing positions and charges.
/// * `force` - Accumulated incoming force vector for particle `i`.
/// * `_settings` - Global simulation parameters (unused in pure Coulomb calculations, preserved for interface uniformity).
///
/// # Returns
///
/// * `DVec3` - The updated force vector including the electrostatic contribution.
pub fn add_coulomb(i: usize, j: usize, particles: &ParticleVec, mut force: DVec3,_settings: &SimulationSettings)-> DVec3{
    const EPS0: f64 = 8.85418782e-12;

    let r = particles.position[i] - particles.position[j];

    let r_mag_sq = r.length_squared();
    let inv_r = 1.0 / r_mag_sq.sqrt(); // One square root
    let inv_r_cubed = inv_r * inv_r * inv_r;

    
    force+=(particles.charge[i] * particles.charge[j] / (4.0 * PI * EPS0)) * r * inv_r_cubed;
    
    force
    
}
