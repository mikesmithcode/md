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
use crate::md_sim::force::check_delta;


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
pub fn add_granular_collision(i: usize, j: usize, particles: &ParticleVec, forces: &mut [DVec3], torques: &mut [DVec3], settings: &SimulationSettings) {   
    // Extract params
    let (stiffness, damping, mu_opt) = match &settings.model {
            SimulationModel::Solid(p) => (p.stiffness, p.damping, None),
            SimulationModel::SolidFriction(p) => (p.stiffness, p.damping, Some(p.mu)),
            _ => panic!("Unsupported model for granular collision"),
        };

    // Calc overlap etc
    let mut delta = particles.position[i] - particles.position[j];
    check_delta(&mut delta, settings.sim_box_size, settings.periodic);

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
pub fn add_weeks_chandler_andersen(i: usize,j: usize,forces: &mut [DVec3], particles: &ParticleVec,settings: &SimulationSettings){

    let mut delta = particles.position[i] - particles.position[j];
    check_delta(&mut delta, settings.sim_box_size, settings.periodic);

    let r2 = delta.x * delta.x + delta.y * delta.y + delta.z * delta.z;
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
                let force_vec = glam::DVec3::new(delta.x * f_mag / r, delta.y * f_mag / r, delta.z * f_mag / r);

                // Add force (equal and opposite occurs because we enter this function with both particles as the i if appropriate.)
                forces[i] += force_vec;

            }
        }

    }

}


pub fn add_coulomb(i: usize, j: usize, particles: &ParticleVec, forces: &mut [DVec3],_settings: &SimulationSettings){
    const EPS0: f64 = 8.85418782e-12;

    let r = particles.position[i] - particles.position[j];

    let r_mag_sq = r.length_squared();
    let inv_r = 1.0 / r_mag_sq.sqrt(); // One square root
    let inv_r_cubed = inv_r * inv_r * inv_r;

    
    forces[i]+=(particles.charge[i] * particles.charge[j] / (4.0 * PI * EPS0)) * r * inv_r_cubed;
    
}
