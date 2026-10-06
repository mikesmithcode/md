//!------------------------------------------------------------------------------
//! Contact Module
//!------------------------------------------------------------------------------
//! Handles contact detection and state tracking for 
//! physical contacts between particles and surfaces in the simulation.

use glam::DVec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use crate::md_sim::utils::file_io::get_latest_file;
use crate::md_sim::utils::check_delta;
use crate::md_sim::particle::ParticleVec;
use crate::md_sim::force::CollisionParams;
use crate::md_sim::SimulationSettings;
use crate::md_sim::particle::{SurfaceKinematics, ObjectSpec};

///------------------------------------------------------------------------------
/// Contact
///------------------------------------------------------------------------------
/// Represents the geometric and kinematic state of an active contact between 
/// two entities (particle-particle or particle-surface).
/// 
/// Fields:
/// * `overlap` - Penetration depth of the contact.
/// * `normal` - Unit normal vector pointing from the interacting entity to the primary particle.
/// * `r_contact` - Radial vector from the centre of the particle to the contact point (used for torque calculation).
/// * `rel_vel` - Relative surface velocity between the contacting bodies at the contact point.
/// * `eff_stiffness` - Combined/effective normal contact stiffness coefficient.
/// * `eff_damping` - Combined/effective normal damping coefficient.
/// * `mu` - Effective friction coefficient for tangential forces.
pub struct Contact{
    pub overlap: f64,
    pub normal: DVec3,
    pub r_contact: DVec3, 
    pub rel_vel: DVec3,
    pub eff_stiffness: f64,
    pub eff_damping: f64,
    pub mu: f64,
}

impl Default for Contact{
    fn default()-> Self{
        Self { 
            overlap: 1e-5, 
            normal: DVec3::new(0.0, 0.0, 1.0), 
            r_contact: DVec3::new(0.0, 0.0, 0.005), 
            rel_vel: DVec3::new(0.0, 0.0, 0.0), 
            eff_stiffness: 1e5, 
            eff_damping: 10.0, 
            mu: 0.5 
        }
    }
}

///------------------------------------------------------------------------------
/// ContactState
///------------------------------------------------------------------------------
/// Tracks history-dependent properties for a specific persistent contact pair 
/// across multiple time steps, such as tangential displacement for friction models.
/// 
/// Fields:
/// * `tangential_disp` - Accumulated tangential displacement vector since contact initiation.
/// * `is_active` - Boolean flag tracking whether the contact was detected and touched during the current time step.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContactState {
    pub tangential_disp: DVec3,
    pub is_active: bool,
}

impl Default for ContactState {
    fn default() -> Self {
        Self { 
            tangential_disp: DVec3::new(1e-5, 0.0, 0.0), 
            is_active: true 
        }
    }
}

///------------------------------------------------------------------------------
/// ContactManager
///------------------------------------------------------------------------------
/// Manages and persists contact states across time steps to support history-dependent 
/// interaction models (e.g., tangential spring-dashpot friction models).
/// 
/// Fields:
/// * `states` - Map storing persistent contact state data keyed by particle index pairs `(i, j)`.
/// * `source_path` - Optional path tracking where the model was loaded from or saved to.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContactManager {
    // We store states as a vector of pairs or use a custom serializer if keeping a HashMap, 
    // because JSON objects require string keys. Storing as a Vec or serializing via sequence 
    // avoids JSON key serialization errors for `(usize, usize)`.
    #[serde(with = "tuple_map_as_vec")]
    pub states: HashMap<(usize, usize), ContactState>,
    
    #[serde(skip)]
    pub source_path: Option<PathBuf>,
}

impl ContactManager {
    ///------------------------------------------------------------------------------
    /// new
    ///------------------------------------------------------------------------------
    /// Initializes an empty `ContactManager` instance with no tracked contacts.
    pub fn new() -> Self {
        Self {
           states: HashMap::new(), 
           source_path: None,
        }
    }

    ///------------------------------------------------------------------------------
    /// load_latest
    ///------------------------------------------------------------------------------
    /// Locates and deserializes the latest contact state file matching the prefix 
    /// from the given directory.
    pub fn load_latest(dir_path: &Path) -> Option<Self> {
        let path = get_latest_file(dir_path, "contacts", "json")?;
        
        let file = File::open(&path).ok()?;
        let reader = BufReader::new(file);
        
        let mut manager: ContactManager = serde_json::from_reader(reader).ok()?;
        manager.source_path = Some(path);
        
        Some(manager)
    }

    ///------------------------------------------------------------------------------
    /// save_at_step
    ///------------------------------------------------------------------------------
    /// Saves contact states to a 10-digit zero-padded step file if a source path exists.
    pub fn save_at_step(&self, step: usize) -> Option<()> {
        let source_path = self.source_path.as_ref()?;

        let parent_dir = source_path.parent().unwrap_or_else(|| Path::new("."));
        let target_path = parent_dir.join(format!("contacts_{:010}.json", step));

        let file = File::create(target_path).ok()?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self).ok()?;
        
        Some(())
    }

    pub fn check_or_add(&mut self, pair: (usize, usize), displacement: DVec3) {
        let state = self.states.entry(pair).or_default();
        state.tangential_disp += displacement;
        state.is_active = true;
    }

    pub fn remove_old_contacts(&mut self) {
        self.states.retain(|_, state| {
            let active = state.is_active;
            state.is_active = false;
            active
        });
    }
}

impl Default for ContactManager {
    fn default() -> Self {
        let mut states = HashMap::new();
        states.insert((0, 1), ContactState::default());

        Self { 
            states, 
            source_path: None,
        }
    }
}

/// Helper module to serialize `HashMap<(usize, usize), ContactState>` as a list of entries 
/// so serde/JSON doesn't crash over non-string map keys.
mod tuple_map_as_vec {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(map: &HashMap<(usize, usize), ContactState>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let vec: Vec<_> = map.iter().collect();
        vec.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<(usize, usize), ContactState>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<((usize, usize), ContactState)> = Vec::deserialize(deserializer)?;
        Ok(vec.into_iter().collect())
    }
}

///------------------------------------------------------------------------------
/// check_particle_contact
///------------------------------------------------------------------------------
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
) -> Option<Contact> { 
    
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

        let contact = Contact {
            overlap,
            normal,
            r_contact: r_i,
            rel_vel, 
            eff_stiffness, 
            eff_damping, 
            mu: model.mu,
        };
        
        Some(contact)
    } else {
        None
    }
}




pub fn check_object_contact(
    i: usize,
    object: &ObjectSpec,
    particles: &ParticleVec,
    model: &CollisionParams,
    settings: &SimulationSettings,
) -> Option<Contact> {
    // Exit early if particle type is excluded from collisions
    if !settings.collision_mask[particles.ptype[i]] {
        return None;
    }

    match object {
        ObjectSpec::Rectangle(rect) => check_surface_contact(i, rect, particles, model, settings),
        ObjectSpec::Triangle(tri) => check_surface_contact(i, tri, particles, model, settings),
        ObjectSpec::WireBox(box_spec) => {
            // Not implemented since this is just a visual element
            None 
        }
        ObjectSpec::Line(line) => {
            // Not implemented since this is just a visual element
            None
        }
    }
}


///------------------------------------------------------------------------------
/// check_surface_contact
///------------------------------------------------------------------------------
/// Calculates collision geometry, relative kinematics, and effective contact properties 
/// between a particle and any rigid object implementing `SurfaceKinematics`.
/// 
/// # Arguments
/// 
/// * `i` - Index of the particle.
/// * `surface` - Reference to any boundary implementing `SurfaceKinematics` (e.g., `RectSpec`, `TriSpec`).
/// * `particles` - Reference to the particle data structure.
/// * `model` - Collision parameter set (`CollisionParams`).
/// * `settings` - Global simulation configuration (`SimulationSettings`).
/// 
/// # Returns
/// 
/// * `Some(Contact)` if the particle is penetrating the surface.
/// * `None` otherwise.
fn check_surface_contact<S: SurfaceKinematics>(
    i: usize,
    surface: &S,
    particles: &ParticleVec,
    model: &CollisionParams,
    settings: &SimulationSettings,
) -> Option<Contact> {

    let closest = surface.closest_point(particles.position[i]);
    let delta = particles.position[i] - closest;
    let dist_sq = delta.length_squared();
    let rad_i = particles.radius[i];

    if dist_sq < rad_i * rad_i {
        let dist = dist_sq.sqrt();
        let normal = if dist > 1e-12 {
            delta / dist
        } else {
            surface.normal()
        };
        let overlap = rad_i - dist;

        let m_eff = particles.mass[i];
        let r_eff = rad_i; // Sphere-plane effective radius limit

        // Radial vector from particle center to the contact point
        let r_i = normal * (-rad_i + overlap);
        let surface_vel = surface.velocity_at_point(closest);

        let rel_vel = (particles.velocity[i] + particles.omega[i].cross(r_i)) - surface_vel;

        // Hertzian parameters from model
        let e_star = model.particle_e_star;
        let beta = model.particle_beta;

        let eff_stiffness = (4.0 / 3.0) * e_star * r_eff.sqrt();
        let eff_damping = 2.0 * beta * (m_eff * eff_stiffness).sqrt();

        let contact = Contact {
            overlap,
            normal,
            r_contact: r_i,
            rel_vel,
            eff_stiffness,
            eff_damping,
            mu: model.mu,
        };

        Some(contact)
    } else {
        None
    }
}