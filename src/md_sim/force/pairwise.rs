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

use crate::md_sim::particle::ParticleVec;
use crate::md_sim::force::contact::{Contact};




///--------------------------------------------------------------------------------------------
/// 
/// Pairwise contact forces
/// 
/// -------------------------------------------------------------------------------------------

///------------------------------------------------------------------------------
/// normal_hertzian
///------------------------------------------------------------------------------
/// Computes the normal Hertzian contact force between two spherical particles.
///
/// This combines a nonlinear Hertzian elastic restoring force ($\delta^{3/2}$) 
/// with a linear viscous damping term. Note that effective radii and material 
/// moduli are implicitly accounted for via the precalculated effective stiffness 
/// (`eff_stiffness`) stored within the contact state.
///
/// The resulting force magnitude is bounded at zero to prevent attractive 
/// (tensile) forces, as standard dry DEM contacts cannot sustain tension.
///
/// # Arguments
///
/// * `contact` - A reference to the active [`Contact`] struct containing overlap depth, 
///               relative velocity, contact normal, and precalculated material/geometric coefficients.
///
/// # Returns
///
/// A tuple containing:
/// * `0`: The scalar magnitude of the normal contact force (`f64`).
/// * `1`: The vector normal force directed along the contact normal (`DVec3`).
#[inline]
pub fn normal_hertzian(contact: &Contact)-> (f64, DVec3){
// Elastic and viscous damping normal forces
    let f_elastic = contact.eff_stiffness * contact.overlap.powf(1.5);
    let f_damping = contact.eff_damping * contact.rel_vel.dot(contact.normal);
    let f_normal_mag = (f_elastic - f_damping).max(0.0);
    
    //f_normal_vec
    (f_normal_mag, contact.normal * f_normal_mag)
}

///------------------------------------------------------------------------------
/// normal_linear
///------------------------------------------------------------------------------
/// Computes the linear normal contact force (linear spring-dashpot model) 
/// between two spherical particles.
///
/// Unlike the Hertzian model, the elastic restoring force is directly 
/// proportional to the overlap depth (`overlap`) rather than $\delta^{3/2}$. 
/// This is combined with a linear viscous damping term based on the normal 
/// component of the relative velocity.
///
/// The resulting force magnitude is bounded at zero to prevent attractive 
/// (tensile) forces, as standard dry DEM contacts cannot sustain tension.
///
/// # Arguments
///
/// * `contact` - A reference to the active `Contact` struct containing overlap depth, 
///               relative velocity, contact normal, and effective stiffness/damping coefficients.
///
/// # Returns
///
/// A tuple containing:
/// * `0`: The scalar magnitude of the normal contact force (`f64`).
/// * `1`: The vector normal force directed along the contact normal (`DVec3`).
#[inline]
pub fn normal_linear(contact: &Contact)-> (f64, DVec3){
    // Elastic and viscous damping normal forces
    let f_elastic = contact.eff_stiffness * contact.overlap;
    let f_damping = contact.eff_damping * contact.rel_vel.dot(contact.normal);
    let f_normal_mag = (f_elastic - f_damping).max(0.0);
    
    //f_normal_vec
    (f_normal_mag, contact.normal * f_normal_mag)
}


///------------------------------------------------------------------------------
/// friction_viscous_damping
///------------------------------------------------------------------------------
/// Computes the tangential contact force using a viscous damping model 
/// constrained by a Coulomb friction limit.
///
/// This calculates the relative tangential velocity by subtracting the normal 
/// component from the total relative velocity vector. If the tangential 
/// velocity is non-zero, a tangential damping force is applied and subsequently 
/// capped by the maximum allowable static/kinetic friction limit ($\mu F_n$).
///
/// # Arguments
///
/// * `f_normal_mag` - The scalar normal force magnitude (`$F_n$`) computed from the normal contact model.
/// * `contact`      - A reference to the active `Contact` struct containing relative velocity, 
///                    contact normal, effective damping coefficient, and friction coefficient ($\mu$).
///
/// # Returns
///
/// A `DVec3` representing the resolved tangential force vector acting on the particle.
pub fn friction_viscous_damping(f_normal_mag: f64, contact: &Contact)-> DVec3{

    let v_tang = contact.rel_vel - contact.rel_vel.dot(contact.normal) * contact.normal;

    if v_tang.length_squared() > 1e-18 {
        let f_t_ideal = v_tang * -contact.eff_damping;
        let limit = contact.mu * f_normal_mag;
        let f_t_mag_sq = f_t_ideal.length_squared();

        if f_t_mag_sq > limit * limit {
            f_t_ideal * (limit / f_t_mag_sq.sqrt())
        } else {
            f_t_ideal
        }
    }
    else{DVec3::ZERO}
}


///------------------------------------------------------------------------------
/// CollisionParams
/// 
/// Configuration parameters for particle-particle and particle-plane contact mechanics.
///------------------------------------------------------------------------------
/// Holds raw material and mechanical properties along with precalculated 
/// derived coefficients used for Hertzian or linear contact force models.
/// 
/// Fields:
/// * `modulus` - Young's modulus of the particle material.
/// * `restitution` - Coefficient of restitution for particle-particle collisions.
/// * `mu` - Friction coefficient for particle-particle contacts.
/// * `plane_modulus` - Young's modulus of the boundary plane material.
/// * `plane_restitution` - Coefficient of restitution for particle-plane collisions.
/// * `plane_mu` - Friction coefficient for particle-plane contacts.
/// * `particle_e_star` - Precalculated effective modulus component for particles.
/// * `particle_beta` - Precalculated damping factor for particles.
/// * `plane_e_star` - Precalculated effective modulus for particle-plane interactions.
/// * `plane_beta` - Precalculated damping factor for particle-plane interactions.
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

/// Temporary raw struct matching JSON input before derived coefficients are computed.
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


///------------------------------------------------------------------------------
/// add_coulomb
///------------------------------------------------------------------------------
/// Calculates and accumulates the electrostatic Coulomb force between two charged 
/// particles within a specified cutoff distance.
///
/// # Arguments
///
/// * `i` - Index of the primary particle.
/// * `j` - Index of the neighboring particle.
/// * `particles` - Reference to the particle state buffers containing positions and charges.
/// * `force` - Accumulated incoming force vector.
/// * `model` - Coulomb interaction parameters including relative permittivity and cutoff.
///
/// # Returns
///
/// * `DVec3` - The updated force vector incorporating the electrostatic contribution.
pub fn add_coulomb(i: usize, j: usize, particles: &ParticleVec, mut force: DVec3, model: &CoulombParams)-> DVec3{
    

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

///------------------------------------------------------------------------------
/// CoulombParams
///------------------------------------------------------------------------------
/// Configuration parameters for electrostatic Coulomb interactions.
///
/// Fields:
/// * `eps_r` - Permittivity parameter (note: $8.854 \times 10^{-12}$ corresponds to vacuum permittivity $\varepsilon_0$).
/// * `cutoff` - Maximum spatial cutoff distance for electrostatic force evaluations.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct CoulombParams{
    pub eps_r: f64,
    pub cutoff: f64,
}

impl Default for CoulombParams{
    fn default()->Self{
        Self{
            eps_r: 1.0,
            cutoff: 0.3,
        }
    }
}

