// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------
//
// Pair Forces - forces applied between particles i and j
//
// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------

use std::default::Default;
use glam::DVec3;
use std::f64::consts::PI;
use serde::{Serialize,Deserialize};

use crate::md_sim::SimulationSettings;
use crate::md_sim::particle::ParticleVec;
use crate::md_sim::utils::check_delta;
use crate::md_sim::force::contact::{Contact};






/// Calculates collision geometry, relative kinematics, and effective contact properties 
/// between two particles. 
/// 
/// N.B. Because each particle is stored in each other's Verlet list (i.e., $i$ knows about $j$ 
/// and $j$ knows about $i$), Newton's third law ($\mathbf{F}_{ij} = -\mathbf{F}_{ji}$) is handled 
/// by evaluating this function symmetrically for both $(i, j)$ and $(j, i)$ pairs.
/// 
/// # Physical Model & Processing Steps
/// 
/// 1. **Collision Mask Filtering**:
///    - Exits early if particle $i$'s type is excluded by `settings.collision_mask`.
/// 2. **Geometry & Overlap**:
///    - Position delta with periodic boundary handling: $\boldsymbol{\delta} = \mathbf{p}_i - \mathbf{p}_j$
///    - Center-to-center distance: $\text{dist} = \Vert{}\boldsymbol{\delta}\Vert{}$
///    - Overlap: $\text{overlap} = (R_i + R_j) - \text{dist}$
///    - Unit normal vector (pointing from $j$ to $i$): $\mathbf{n} = \frac{\boldsymbol{\delta}}{\text{dist}}$
/// 3. **Effective Properties & Hertzian Contact Parameters**:
///    - Reduced mass: $m_{\text{eff}} = \begin{cases} \frac{m_i m_j}{m_i + m_j} & \text{if } j \text{ is a collision object} \\ m_i & \text{otherwise} \end{cases}$
///    - Effective radius: $r_{\text{eff}} = \frac{R_i R_j}{R_i + R_j}$
///    - Contact stiffness ($k_n$): $k_n = \frac{4}{3} E^* \sqrt{r_{\text{eff}}}$
///    - Damping coefficient ($\gamma_n$): $\gamma_n = 2 \beta \sqrt{m_{\text{eff}} k_n}$
/// 4. **Kinematics & Contact Points**:
///    - Contact offset vectors: $\mathbf{r}_i = \mathbf{n} \left(-R_i + \frac{\text{overlap} \cdot R_j}{R_i + R_j}\right)$, $\mathbf{r}_j = \mathbf{n} \left(R_j - \frac{\text{overlap} \cdot R_i}{R_i + R_j}\right)$
///    - Relative surface velocity: $\mathbf{v}_{\text{rel}} = (\mathbf{v}_i + \boldsymbol{\omega}_i \times \mathbf{r}_i) - (\mathbf{v}_j + \boldsymbol{\omega}_j \times \mathbf{r}_j)$
/// 
/// # Arguments
/// 
/// * `i`, `j` - Indices of the interacting particles.
/// * `particles` - Reference to the particle data structure (position, velocity, omega, radius, mass, type).
/// * `model` - Collision parameter set (`CollisionParams`) containing elastic modulus, damping factors, and friction coefficients.
/// * `settings` - Global simulation configuration (`SimulationSettings`) for box size and periodicity.
/// 
/// # Returns
/// 
/// * `Some(Contact)` containing the resolved geometry, kinematics, and material properties if overlapping.
/// * `None` if particles are separated or filtered out by collision masks.
pub fn check_particle_contact(
    i: usize, 
    j: usize, 
    particles: &ParticleVec, 
    model: &CollisionParams, 
    settings: &SimulationSettings
) -> Option<Contact>{ 
    
    // exit if not a collision type
    if !settings.collision_mask[particles.ptype[i]] {
        return None;
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

        let is_j_coll = settings.collision_mask[particles.ptype[j]];      
        
        let m_i = particles.mass[i];
        let m_eff = if is_j_coll {
            let m_j = particles.mass[j];
            (m_i * m_j) / (m_i + m_j)
        } else {
            m_i
        };

        let r_eff = (rad_i * rad_j) / combined_rad;

        let r_i = normal * (-rad_i + overlap * rad_j / combined_rad);
        let r_j = normal * (rad_j - overlap * rad_i / combined_rad);

        let rel_vel = (particles.velocity[i] + particles.omega[i].cross(r_i)) 
                            - (particles.velocity[j] + particles.omega[j].cross(r_j));

        // Using precomputed values from model
        let e_star = model.particle_e_star;
        let beta = model.particle_beta;

        let eff_stiffness = (4.0 / 3.0) * e_star * r_eff.sqrt();
        let eff_damping = 2.0 * beta * (m_eff * eff_stiffness).sqrt();

        let contact = Contact{overlap,normal,r_contact: r_i,rel_vel, eff_stiffness, eff_damping, mu: model.mu};
        
        Some(contact)
    }else{
        None
    }

}


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(from = "RawCollisionParams")] // Tells Serde to parse via the raw struct first
pub struct CollisionParams {
    pub modulus: f64,
    pub restitution: f64,
    pub mu: f64,
    pub plane_modulus: f64,
    pub plane_restitution: f64,
    pub plane_mu: f64,
    
    pub particle_e_star: f64,
    pub particle_beta: f64,
    pub plane_e_star: f64,
    pub plane_beta: f64,
}

impl Default for CollisionParams {
    fn default() -> Self {
        // Define standard default values for your raw parameters
        let raw = RawCollisionParams {
            modulus: 1.0E6,       
            restitution: 0.8,     
            mu: 0.5,              
            plane_modulus: 1.0E6, 
            plane_restitution: 0.8,
            plane_mu: 0.5,
        };

        // Automatically compute the derived fields using your existing From implementation
        Self::from(raw)
    }
}

// Temporary raw struct matching JSON input
#[derive(Deserialize)]
struct RawCollisionParams {
    pub modulus: f64,
    pub restitution: f64,
    pub mu: f64,
    pub plane_modulus: f64,
    pub plane_restitution: f64,
    pub plane_mu: f64,
}

// Automatically compute precalculated values when converting from raw to CollisionParams
impl From<RawCollisionParams> for CollisionParams {
    fn from(raw: RawCollisionParams) -> Self {
        let particle_e_star = raw.modulus / 1.82;
        let particle_beta = -raw.restitution.ln() 
            / (std::f64::consts::PI.powi(2) + raw.restitution.ln().powi(2)).sqrt();

        let compliance = 0.91 * ((1.0 / raw.modulus) + (1.0 / raw.plane_modulus));
        let plane_e_star = 1.0 / compliance;

        let combined_restitution = (raw.restitution * raw.plane_restitution).sqrt();
        let plane_beta = -combined_restitution.ln() 
            / (std::f64::consts::PI.powi(2) + combined_restitution.ln().powi(2)).sqrt();

        Self {
            modulus: raw.modulus,
            restitution: raw.restitution,
            mu: raw.mu,
            plane_modulus: raw.plane_modulus,
            plane_restitution: raw.plane_restitution,
            plane_mu: raw.plane_mu,
            particle_e_star,
            particle_beta,
            plane_e_star,
            plane_beta,
        }
    }
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
/// * `model` - Global simulation parameters (unused in pure Coulomb calculations, preserved for interface uniformity).
///
/// # Returns
///
/// * `DVec3` - The updated force vector including the electrostatic contribution.
pub fn add_coulomb(i: usize, j: usize, particles: &ParticleVec, mut force: DVec3, model: CoulombParams)-> DVec3{
    

    let r = particles.position[i] - particles.position[j];
    let r_mag_sq = r.length_squared();
    
    // Implement cutoff
    if r_mag_sq <= model.cutoff.powi(2){
        const EPS0: f64 = 8.85418782e-12;
        let eps = EPS0 * model.eps_r;
        let inv_r = 1.0 / r_mag_sq.sqrt(); // One square root
        let inv_r_cubed = inv_r * inv_r * inv_r;
    
        force+=(particles.charge[i] * particles.charge[j] / (4.0 * PI * eps)) * r * inv_r_cubed;
    }
    force
    
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct CoulombParams{
    pub eps_r: f64,
    pub cutoff: f64,
}

impl Default for CoulombParams{
    fn default()->Self{
        Self{
            eps_r: 8.854E-12,
            cutoff: 0.3,
        }
    }
}

