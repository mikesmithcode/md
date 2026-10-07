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
use crate::md_sim::force::contact::{Contact, ContactType, ContactManager};




///============================================================================================
/// 
/// Pairwise contact forces
/// 
///============================================================================================
///============================================================================================
/// normal_hertzian
///============================================================================================
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
pub fn normal_hertzian(
    contact: &Contact, 
    model: &NormalForce
) -> (f64, DVec3) {
    // Select the appropriate precomputed material constants based on contact type
    let (e_star, beta) = match contact.contact_type {
        ContactType::Particle => (model.particle_estar, model.particle_beta),
        ContactType::Plane => (model.plane_estar, model.plane_beta),
    };

    // Compute local effective stiffness and damping using geometry/mass from Contact
    let eff_stiffness = (4.0 / 3.0) * e_star * contact.r_eff.sqrt();
    let eff_damping = 2.0 * beta * (contact.m_eff * eff_stiffness).sqrt();

    // Elastic and viscous damping normal forces
    let f_elastic = eff_stiffness * contact.overlap.powf(1.5);
    let f_damping = eff_damping * contact.rel_vel.dot(contact.normal);
    let f_normal_mag = (f_elastic - f_damping).max(0.0);

    // f_normal_vec
    (f_normal_mag, contact.normal * f_normal_mag)
}

///============================================================================================
/// normal_linear
///============================================================================================
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
pub fn normal_linear(
    contact: &Contact, 
    model: &NormalForce
) -> (f64, DVec3) {
    // Select the appropriate precomputed material constants based on contact type
    let (e_star, beta) = match contact.contact_type {
        ContactType::Particle => (model.particle_estar, model.particle_beta),
        ContactType::Plane => (model.plane_estar, model.plane_beta),
    };

    // Correct dimensional linear stiffness: 
    // [E*] (N/m^2) * [r_eff] (m) = N/m (Linear stiffness units)
    let eff_stiffness = 2.0 * e_star * contact.r_eff; 
    let eff_damping = 2.0 * beta * (contact.m_eff * eff_stiffness).sqrt();

    // Linear elastic force (F = k * delta) and viscous damping
    let f_elastic = eff_stiffness * contact.overlap;
    let f_damping = eff_damping * contact.rel_vel.dot(contact.normal);
    let f_normal_mag = (f_elastic - f_damping).max(0.0);

    (f_normal_mag, contact.normal * f_normal_mag)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(from = "RawNormalForce")]
pub struct NormalForce {
    pub modulus: f64,
    pub restitution: f64,
    pub plane_modulus: f64,
    pub plane_restitution: f64,
    #[serde(skip)]
    pub particle_estar: f64,
    #[serde(skip)]
    pub particle_beta: f64,
    #[serde(skip)]
    pub plane_estar: f64,
    #[serde(skip)]
    pub plane_beta: f64,
}

// Raw struct matching the exact keys found in your JSON input
#[derive(Deserialize)]
pub(crate) struct RawNormalForce {
    pub modulus: f64,
    pub restitution: f64,
    pub plane_modulus: f64,
    pub plane_restitution: f64,
}

impl From<RawNormalForce> for NormalForce {
    fn from(raw: RawNormalForce) -> Self {
        // Assume standard Poisson's ratio nu = 0.25
        let nu = 0.25;
        let denom = 1.0 - nu * nu;

        // Particle-Particle Effective Modulus (assuming identical particle materials)
        // 1 / E* = (1-nu^2)/E + (1-nu^2)/E = 2(1-nu^2)/E
        let particle_estar = raw.modulus / (2.0 * denom);

        // Particle-Plane Effective Modulus (combining particle and plane moduli)
        // 1 / E* = (1-nu^2)/E_particle + (1-nu^2)/E_plane
        let compliance_particle = denom / raw.modulus;
        let compliance_plane = denom / raw.plane_modulus;
        let plane_estar = 1.0 / (compliance_particle + compliance_plane);

        // Damping factors derived from coefficients of restitution (e)
        // beta = -ln(e) / sqrt(pi^2 + ln(e)^2)
        let particle_beta = if raw.restitution > 0.0 && raw.restitution < 1.0 {
            let ln_e = raw.restitution.ln();
            -ln_e / (std::f64::consts::PI.powi(2) + ln_e.powi(2)).sqrt()
        } else {
            0.0
        };

        let plane_beta = if raw.plane_restitution > 0.0 && raw.plane_restitution < 1.0 {
            let ln_e = raw.plane_restitution.ln();
            -ln_e / (std::f64::consts::PI.powi(2) + ln_e.powi(2)).sqrt()
        } else {
            0.0
        };

        Self {
            modulus: raw.modulus,
            restitution: raw.restitution,
            plane_modulus: raw.plane_modulus,
            plane_restitution: raw.plane_restitution,
            particle_estar,
            particle_beta,
            plane_estar,
            plane_beta,
        }
    }
}

impl Default for NormalForce {
    fn default() -> Self {
        RawNormalForce {
            modulus: 1e5,
            restitution: 0.5,
            plane_modulus: 1e5,
            plane_restitution: 0.5,
        }
        .into()
    }
}

///============================================================================================
/// friction_viscous_damping
///============================================================================================
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
#[inline]
pub fn friction_viscous_damping(
    f_normal_mag: f64, 
    contact: &Contact, 
    model: &FrictionViscous
) -> DVec3 {
    let v_tang = contact.rel_vel - contact.rel_vel.dot(contact.normal) * contact.normal;

    if v_tang.length_squared() > 1e-18 {
        // Compute tangential damping using the parameter struct's value 
        // (or combine it with m_eff / stiffness if you want it dynamically scaled)
        let c_t = model.damping_coeff; // or derived from contact.m_eff and model parameters
        
        let f_t_ideal = v_tang * -c_t;
        let limit = model.mu * f_normal_mag;
        let f_t_mag_sq = f_t_ideal.length_squared();

        if f_t_mag_sq > limit * limit {
            f_t_ideal * (limit / f_t_mag_sq.sqrt())
        } else {
            f_t_ideal
        }
    } else {
        DVec3::ZERO
    }
}


///============================================================================================
/// FrictionViscous
///============================================================================================
/// Configuration parameters for the viscous tangential damping model.
///
/// Stores user-facing configuration fields (`mu`, `restitution`) which serialize and 
/// deserialize directly, while precomputing the simulation-ready `damping_coeff` upon deserialization.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(from = "RawFrictionViscous")]
pub struct FrictionViscous {
    pub mu: f64,
    pub restitution: f64,
    #[serde(skip)]
    pub damping_coeff: f64,
}

///============================================================================================
/// RawFrictionViscous
///============================================================================================
/// Intermediate configuration struct used to parse raw JSON inputs for the viscous friction model.
#[derive(Deserialize)]
struct RawFrictionViscous {
    pub mu: f64,
    pub restitution: f64,
}

impl From<RawFrictionViscous> for FrictionViscous {
    fn from(raw: RawFrictionViscous) -> Self {
        let damping_coeff = if raw.restitution > 0.0 && raw.restitution < 1.0 {
            let ln_e = raw.restitution.ln();
            -ln_e / (std::f64::consts::PI.powi(2) + ln_e.powi(2)).sqrt()
        } else {
            0.0
        };

        Self {
            mu: raw.mu,
            restitution: raw.restitution,
            damping_coeff,
        }
    }
}

impl Default for FrictionViscous {
    fn default() -> Self {
        let mu:f64 = 0.5;
        let restitution:f64 = 0.5;
        
        let damping_coeff = if restitution > 0.0 && restitution < 1.0 {
            let ln_e = restitution.ln();
            -ln_e / (std::f64::consts::PI.powi(2) + ln_e.powi(2)).sqrt()
        } else {
            0.0
        };

        Self {
            mu,
            restitution,
            damping_coeff,
        }
    }
}

///============================================================================================
/// friction_cundall_strack
///============================================================================================
/// Computes the tangential contact force using the Cundall-Strack history-dependent 
/// spring-dashpot model constrained by a Coulomb friction limit.
///
/// Accumulates tangential shear displacement across time steps via the thread-safe `ContactManager`, 
/// applying an elastic restoring spring force and a viscous tangential damping force, 
/// bounded by the maximum friction yield limit ($\mu F_n$). If sliding occurs, the stored 
/// tangential displacement history is back-corrected to prevent unphysical accumulation.
///
/// # Arguments
///
/// * `f_normal_mag`    - The scalar normal force magnitude ($F_n$) computed from the normal contact model.
/// * `contact`         - A reference to the active `Contact` struct containing spatial geometry and relative velocities.
/// * `model`           - A reference to the `FrictionCundallStrack` configuration parameters.
/// * `contact_manager` - A reference to the thread-safe `ContactManager` tracking history-dependent states.
/// * `i`               - Index of the first entity in the contact pair.
/// * `j`               - Index of the second entity in the contact pair.
/// * `dt`              - The simulation time step duration.
///
/// # Returns
///
/// A `DVec3` representing the resolved tangential force vector acting on the particle.
pub fn friction_cundall_strack(
    f_normal_mag: f64,
    contact: &Contact,
    model: &FrictionCundallStrack,
    contact_manager: &ContactManager,
    i: usize,
    j: usize,
    dt: f64,
) -> DVec3 {
    let pair = (i, j);

    // Extract tangential relative velocity component (reusing your clean vector projection)
    let v_tang = contact.rel_vel - contact.rel_vel.dot(contact.normal) * contact.normal;

    // Compute incremental tangential displacement and update history in the manager
    let delta_tangential = v_tang * dt;
    contact_manager.check_or_add(pair, delta_tangential);

    // Retrieve accumulated tangential displacement
    let full_tang_disp = contact_manager.states.get(&pair).unwrap().tangential_disp;

    // Define tangential stiffness and damping coefficients 
    // (Standard Hertz-Mindlin convention: k_t is typically 2/3 of normal stiffness)
    let k_t = (2.0 / 3.0) * model.tang_stiffness;
    let c_t = 0.5 * model.tang_damping;

    // Calculate trial tangential force (Elastic Spring + Viscous Damping)
    let mut f_t_ideal = -k_t * full_tang_disp - c_t * v_tang;

    // Apply Coulomb friction yield limit (||F_t|| <= mu * F_n)
    let limit = model.mu * f_normal_mag;
    let f_t_mag_sq = f_t_ideal.length_squared();

    if f_t_mag_sq > limit * limit && f_t_mag_sq > 1e-18 {
        let f_t_mag = f_t_mag_sq.sqrt();
        f_t_ideal = f_t_ideal * (limit / f_t_mag);

        // Back-correct stored tangential displacement so history doesn't 
        // falsely over-accumulate while sliding under the friction limit
        if let Some(mut state) = contact_manager.states.get_mut(&pair) {
            state.tangential_disp = -f_t_ideal / k_t;
        }
    }

    f_t_ideal
}

///============================================================================================
/// FrictionCundallStrack
///============================================================================================
/// Configuration parameters for the Cundall-Strack tangential contact model.
///
/// Stores resolved tangential stiffness, damping coefficients, and the Coulomb friction 
/// coefficient ($\mu$), derived automatically from raw user configuration via Serde.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(from = "RawFrictionCundallStrack")]
pub struct FrictionCundallStrack {
    pub modulus: f64,
    pub restitution: f64,
    pub mu: f64,
    #[serde(skip)]
    pub tang_stiffness: f64,
    #[serde(skip)]
    pub tang_damping: f64,
}

///============================================================================================
/// RawFrictionCundallStrack
///============================================================================================
/// Intermediate configuration struct used to parse raw JSON inputs for the Cundall-Strack model.
///
/// Allows users to specify a dedicated tangential modulus or fall back to standard 
/// proportions relative to the normal contact parameters.
///============================================================================================
/// RawFrictionCundallStrack
///============================================================================================
/// Intermediate configuration struct used to parse raw JSON inputs for the Cundall-Strack model.
#[derive(Deserialize)]
struct RawFrictionCundallStrack {
    pub modulus: f64,
    pub restitution: f64,
    pub mu: f64,
}

impl From<RawFrictionCundallStrack> for FrictionCundallStrack {
    fn from(raw: RawFrictionCundallStrack) -> Self {
        let tang_stiffness = raw.modulus;

        // Derive tangential damping using the standard restitution relation
        let tang_damping = if raw.restitution > 0.0 && raw.restitution < 1.0 {
            let ln_e = raw.restitution.ln();
            -ln_e / (std::f64::consts::PI.powi(2) + ln_e.powi(2)).sqrt() * tang_stiffness
        } else {
            0.0
        };

        Self {
            modulus: raw.modulus,
            restitution: raw.restitution,
            mu: raw.mu,
            tang_stiffness,
            tang_damping,
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

