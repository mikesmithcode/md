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
use crate::md_sim::force::common::compute_contact_force_and_torque;


/// Calculates contact forces and torques between two particles using a Linear Spring-Dashpot (LSD) model.
/// This function handles both central repulsion (normal force) and optional surface friction
/// (tangential force). It accounts for rotational dynamics by calculating relative velocity
/// at the contact point and applying resulting torques.
/// N.B because each particle is stored in each others Verlet list (ie i knows about j and j knows about i)
/// when an interaction is possible we don't apply Newton's third law (ie $F_ij = -F_ji$). This is done
/// by running this function for both i, j and j,i.
/// 
/// # Physical Model
/// 
/// 1. Geometry & Overlap:
///     - $\mathbf{\delta} = \mathbf{p}_i - \mathbf{p}_j$ (vector from particle $j$ center to particle $i$ center)
///     - $\text{dist} = \Vert{}\mathbf{\delta}\Vert{}$, $\text{overlap} = (R_i + R_j) - \text{dist}$
///     - $\mathbf{n} = \frac{\mathbf{\delta}}{\text{dist}}$ (unit vector pointing from $j$ to $i$)
/// 2. Effective Properties:
///     - Reduced mass: $m_{\text{eff}} = \frac{m_i m_j}{m_i + m_j}$ (or $m_i$ if $j$ is a non-collision boundary)
///     - Effective radius: $r_{\text{eff}} = \frac{R_i R_j}{R_i + R_j}$
///     - Contact stiffness ($k_n$) & damping ($\gamma_n$) derived via Hertzian-linear approximation.
/// 3. Normal Force ($\mathbf{F}_n$):
///     - Relative normal velocity: $v_n = (\mathbf{v}_i - \mathbf{v}_j) \cdot \mathbf{n}$ (negative during compression, positive during separation)
///     - Clamped magnitude: $F_{n,\text{mag}} = \max\left(0, k_n \cdot \text{overlap} - \gamma_n v_n\right)$
///     - Vector normal force: $\mathbf{F}_n = F_{n,\text{mag}} \mathbf{n}$ (pointing from $j$ to $i$)
/// 4. Tangential Friction Force ($\mathbf{F}_t$):
///     - Contact points offset: $\mathbf{r}_i = \mathbf{n} \left(-R_i + \frac{\text{overlap} \cdot R_j}{R_i + R_j}\right)$, $\mathbf{r}_j = \mathbf{n} \left(R_j - \frac{\text{overlap} \cdot R_i}{R_i + R_j}\right)$
///     - Surface relative velocity: $\mathbf{v}_{\text{surface\_rel}} = (\mathbf{v}_i + \boldsymbol{\omega}_i \times \mathbf{r}_i) - (\mathbf{v}_j + \boldsymbol{\omega}_j \times \mathbf{r}_j)$
///     - Tangential velocity: $\mathbf{v}_{\text{tang}} = \mathbf{v}_{\text{surface\_rel}} - (\mathbf{v}_{\text{surface\_rel}} \cdot \mathbf{n})\mathbf{n}$
///     - Ideal viscous friction: $\mathbf{F}_{t,\text{ideal}} = -\gamma_n \mathbf{v}_{\text{tang}}$
///     - Coulomb friction limit: $F_{\text{limit}} = \mu F_{n,\text{mag}}$
///     - Clamped friction vector: $\mathbf{F}_t = \min\left(1, \frac{F_{\text{limit}}}{\Vert{}\mathbf{F}_{t,\text{ideal}}\Vert{}}\right) \mathbf{F}_{t,\text{ideal}}$
/// 5. Induced Torque ($\boldsymbol{\tau}$):$$\boldsymbol{\tau} = \mathbf{r}_i \times \mathbf{F}_t$$
/// 
/// # Arguments
/// 
/// * i, j - Indices of the interacting particles.
/// * particles - Reference to the particle data structure (includes position, velocity, and omega).
/// * force - Accumulator for linear forces. Modified and returned.
/// * torque - Accumulator for angular torques. Modified and returned.
/// * settings - Global simulation config, including the SimulationModel for parameter dispatch.
/// 
/// # Periodic Boundaries
/// 
/// * Minimum Image Convention: Automatically handles periodic wrapping via check_delta
/// to ensure interactions occur over the shortest path across boundaries. check_delta handles
/// whether a boundary is periodic or not and changes behavior accordingly.
/// 
/// # Performance
/// 
/// Marked #[inline(always)] to facilitate compiler optimisations within the spatial
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
    
    let is_i_coll = settings.collision_ptypes.contains(&(particles.ptype[i] as u8));
    if !is_i_coll {
        return (force, torque);
    }

    let mut delta = particles.position[i] - particles.position[j];
    check_delta(&mut delta, settings.sim_box_size, settings.periodic);

    let rad_i = particles.radius[i];
    let rad_j = particles.radius[j];
    let combined_rad = rad_i + rad_j;
    let dist_sq = delta.length_squared();

    if dist_sq < combined_rad * combined_rad {
        let dist = dist_sq.sqrt();
        let normal = delta / dist; 
        let overlap = combined_rad - dist; 

        let is_j_coll = settings.collision_ptypes.contains(&(particles.ptype[j] as u8));      
        
        //If we make a particle static or move with fixed rules behaves like particle
        // of infinite mass so mass in collision just particle i's mass.
        let m_i = particles.mass[i];
        let m_scale = if !is_j_coll {
            1.0
        } else {
            let m_j = particles.mass[j];
            m_j / (m_i + m_j)
        };

        let m_eff = m_scale * m_i;

        let (modulus, restitution, mu) = match &settings.model {
            SimulationModel::Frictional(p) => (p.modulus, p.restitution, p.mu),
            _ => panic!("Unsupported model for granular collision"),
        };

        // 1/E* = (1-nu_i^2)/Yi + (1-nu_j^2)/Yj. Assume nu = 0.3 and Yi=Yj 
        let e_star = modulus / 1.82;
        let beta = -restitution.ln() / (std::f64::consts::PI.powi(2) + restitution.ln().powi(2)).sqrt();

        // Hertzian contact, stiffness depends on radii
        let r_eff = (rad_i * rad_j) / combined_rad;
        let eff_stiffness = (4.0 / 3.0) * e_star * r_eff.sqrt();
        let eff_damping = 2.0 * beta * (m_eff * eff_stiffness).sqrt();
        //println!("particle stiffness {}, damping {}", eff_stiffness, eff_damping);

        let r_i = normal * (-rad_i + overlap * rad_j / combined_rad);
        let r_j = normal * (rad_j - overlap * rad_i / combined_rad);

        let v_surface_rel = (particles.velocity[i] + particles.omega[i].cross(r_i)) 
                            - (particles.velocity[j] + particles.omega[j].cross(r_j));

        let (contact_force, contact_torque) = compute_contact_force_and_torque(
            overlap, normal, r_i, v_surface_rel, eff_stiffness, eff_damping, mu
        );
        
        force += contact_force;
        torque += contact_torque;
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
    
    let df = (particles.charge[i] * particles.charge[j] / (4.0 * PI * EPS0)) * r * inv_r_cubed;
    
    force+=(particles.charge[i] * particles.charge[j] / (4.0 * PI * EPS0)) * r * inv_r_cubed;
    
    force
    
}
