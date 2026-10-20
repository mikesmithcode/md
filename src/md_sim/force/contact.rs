//!------------------------------------------------------------------------------
//! Contact Module
//!------------------------------------------------------------------------------
//! Handles contact detection and state tracking for 
//! physical contacts between particles and surfaces in the simulation.

use glam::DVec3;
use serde::{Deserialize, Serialize, Deserializer, Serializer};
use dashmap::DashMap;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::hash::Hash;


use crate::md_sim::force::contact;
use crate::md_sim::motion::MoleculeData;
use crate::md_sim::utils::file_io::get_latest_file;
use crate::md_sim::utils::check_delta;
use crate::md_sim::particle::ParticleVec;
use crate::md_sim::SimulationSettings;
use crate::md_sim::particle::{SurfaceKinematics, ObjectSpec};
use crate::md_sim::utils::file_io::SimulationContext;

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
/// * `r_eff` - effective rad curvature 1/r_eff = 1/r1 + 1/r2,
/// * `m_eff` - Effective mass 1/m_eff = 1/m1+1/m2,
/// * `contact_type` - [`ContactType`] enum defining whether contact is with particle or plane,
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Contact{
    pub overlap: f64,
    pub tangential_disp: DVec3,
    pub normal: DVec3,
    pub r_contact: DVec3, 
    pub rel_vel: DVec3,
    pub r_eff: f64,
    pub m_eff: f64,
    pub contact_type: ContactType,
    pub is_active: bool,
}

impl Contact  {
    pub fn v_tang(&self)-> DVec3{
        self.rel_vel - self.rel_vel.dot(self.normal) * self.normal
    }
    
}

impl Default for Contact{
    fn default()-> Self{
        Self { 
            overlap: 1e-5, 
            tangential_disp: DVec3::ZERO,
            normal: DVec3::new(0.0, 0.0, 1.0), 
            r_contact: DVec3::new(0.0, 0.0, 0.005), 
            rel_vel: DVec3::new(0.0, 0.0, 0.0),
            r_eff: 0.0,
            m_eff: 0.0,
            contact_type: ContactType::Particle{i:0,j:1},
            is_active: true,
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContactType{
    Particle{i:usize,j:usize},
    Plane{i:usize,plane_id: usize},
}

//Make sure smallest index stored is always i
impl ContactType {
    pub fn particle(i: usize, j: usize) -> Self {
        // Always store the smaller index first so (i, j) and (j, i) map to the exact same entry
        if i < j {
            ContactType::Particle { i, j }
        } else {
            ContactType::Particle { i: j, j: i }
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
    
    #[serde(with = "dashmap_as_vec")]
    pub states: DashMap<ContactType, Contact>,
}

impl ContactManager {
    /// ==============================================================================
    /// new
    /// ==============================================================================
    /// Inititialises a fresh contact manager
    pub fn new() -> Self {
        Self {
            states: DashMap::new(),
        }
    }

    ///------------------------------------------------------------------------------
    /// load_latest
    ///------------------------------------------------------------------------------
    /// Locates and deserializes the latest contact state file matching the prefix 
    /// from the given directory.
    pub fn load_latest(ctx: &SimulationContext) -> Option<Self> {
        let config_dir = ctx.paths.output.join("config");
        let path = get_latest_file(&config_dir, "contacts", "json")?;
        
        let file = File::open(&path).ok()?;
        let reader = BufReader::new(file);
        
        let manager: ContactManager = serde_json::from_reader(reader).ok()?;
        
        Some(manager)
    }

    ///------------------------------------------------------------------------------
    /// save_at_step
    ///------------------------------------------------------------------------------
    /// Saves contact states to a 10-digit zero-padded step file
    pub fn save_at_step(&self, ctx :&SimulationContext, step: usize) -> Option<()> {
        let config_dir = ctx.paths.output.join("config");
        let target_path = config_dir.join(format!("contacts_{:010}.json", step));
        let file = File::create(target_path).ok()?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self).ok()?;
        
        Some(())
    }

    
    pub fn analyse_particle_contact(&self,
        i: usize,
        j: usize,
        particles: &ParticleVec,
        settings: &SimulationSettings,
    ) -> Option<Contact> {
        // Get geometry (returns None if no collision)
        let contact = compute_particle_contact(i, j, particles, settings)?;

        // Compute incremental tangential displacement for this step
        let delta_tangential = contact.v_tang() * settings.dt;

        let contact = self.add_or_update_contact_history(contact, delta_tangential);

        Some(contact)
    
    }

    /// Adds a new contact or updates an existing one by calculating the tangential displacement and adding this
    /// to the current total.
    pub fn add_or_update_contact_history(&self, mut contact: Contact, delta_tangential: DVec3) -> Contact{
        let c_type = contact.contact_type.clone();
        
        let entry = self.states.entry(c_type);
        match entry {
            dashmap::mapref::entry::Entry::Occupied(mut occ) => {
                let existing = occ.get_mut();
                existing.tangential_disp += delta_tangential;
                existing.is_active = true;
                contact.tangential_disp = existing.tangential_disp;
            }
            dashmap::mapref::entry::Entry::Vacant(vac) => {
                contact.tangential_disp = delta_tangential;
                contact.is_active = true;
                vac.insert(contact.clone());
            }
        }
        contact
    }
    

    /// if is_active has been set to false, contact has finished and needs cleaning up
    /// on_separation is a func describing what else to do at this point. You are passing
    /// a closure so it encapsulates the environment of the calling function.
    pub fn prune_contacts<F>(&self, mut on_separation: F)
    where
        F: FnMut(&ContactType, &Contact),
    {
        self.states.retain(|contact_type, state| {
            
            if !state.is_active {
                on_separation(contact_type, state);
                false
            } else {
                state.is_active = false; // reset for the next step
                true // keep it
            }
        });
    }  

    /// Default pruning when no custom separation events (like charging) are needed.
    pub fn remove_old_contacts(&self) {
        self.prune_contacts(|_, _| {});
    }
}

// Helper method for serializing and deserializing Contactmanager's states field.
pub mod dashmap_as_vec {
    use super::*;

    pub fn serialize<S, K, V>(map: &DashMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        K: Serialize + Clone + Eq + Hash, // <- Added Eq + Hash here
        V: Serialize + Clone,
    {
        // Collect dashmap entries into a vector of key-value pairs
        let vec: Vec<(K, V)> = map.iter()
            .map(|item| (item.key().clone(), item.value().clone()))
            .collect();
        vec.serialize(serializer)
    }

    pub fn deserialize<'de, D, K, V>(deserializer: D) -> Result<DashMap<K, V>, D::Error>
    where
        D: Deserializer<'de>,
        K: Deserialize<'de> + Eq + Hash,
        V: Deserialize<'de>,
    {
        // Deserialize into a vector of pairs, then insert into a new DashMap
        let vec: Vec<(K, V)> = Vec::deserialize(deserializer)?;
        let map = DashMap::new();
        for (k, v) in vec { 
            map.insert(k, v);
        }
        Ok(map)
    }
}




///------------------------------------------------------------------------------
    /// compute_particle_contact
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
    pub fn compute_particle_contact(
        i: usize, 
        j: usize, 
        particles: &ParticleVec, 
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

            let contact_type = ContactType::particle(i, j);

            let contact = Contact {
                overlap,
                tangential_disp: DVec3::ZERO,
                normal,
                r_contact: r_i,
                rel_vel, 
                r_eff,
                m_eff,
                contact_type,//Overwritten by impl ContactManager {fn check_or_add}
                is_active: true,//Overwritten by impl ContactManager {fn check_or_add}
            };
            
            Some(contact)
        } else {
            None
        }
    }




    pub fn compute_object_contact(
        i: usize,
        object: &ObjectSpec,
        particles: &ParticleVec,
        settings: &SimulationSettings,
    ) -> Option<Contact> {
        // Exit early if particle type is excluded from collisions
        if !settings.collision_mask[particles.ptype[i]] {
            return None;
        }

        match object {
            ObjectSpec::Rectangle(rect) => compute_surface_contact(i, rect, particles),
            ObjectSpec::Triangle(tri) => compute_surface_contact(i, tri, particles),
            ObjectSpec::WireBox(_box_spec) => {
                // Not implemented since this is just a visual element
                None 
            }
            ObjectSpec::Line(_line) => {
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
    fn compute_surface_contact<S: SurfaceKinematics>(
        i: usize,
        surface: &S,
        particles: &ParticleVec
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

            let contact = Contact {
                overlap,
                normal,
                r_contact: r_i,
                rel_vel,
                r_eff,
                m_eff,
                contact_type: ContactType::Plane { i, plane_id: surface.id() },
                tangential_disp: DVec3::ZERO, //Overwritten by impl ContactManager {fn check_or_add}
                is_active: true, //Overwritten by impl ContactManager {fn check_or_add}
            };

            Some(contact)
        } else {
            None
        }
    }
