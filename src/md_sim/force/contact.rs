//!------------------------------------------------------------------------------
//! Contact Module
//!------------------------------------------------------------------------------
//! Handles contact detection and state tracking for 
//! physical contacts between particles and surfaces in the simulation.

use glam::DVec3;
use serde::{Deserialize, Serialize, Deserializer, Serializer};
use dashmap::DashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::hash::Hash;


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
/// * `r_i` - Radial vector from the centre of the particle to the contact point (used for torque calculation).
/// * `r_j` - Radial vector from centre of other particle to contact point. If the contact is with a surface this is DVec3::ZERO
/// * `rel_vel` - Relative surface velocity between the contacting bodies at the contact point.
/// * `r_eff` - effective rad curvature 1/r_eff = 1/r1 + 1/r2,
/// * `m_eff` - Effective mass 1/m_eff = 1/m1+1/m2,
/// * `contact_type` - [`ContactType`] enum defining whether contact is with particle or plane,
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Contact{
    pub overlap: f64,
    pub tangential_disp: DVec3,
    pub normal: DVec3,
    pub r_i: DVec3, 
    pub r_j: DVec3,
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
    
    // Send the particle of interest i
    pub fn get_contact(&self, query_i: usize) -> Self {
        match self.contact_type {
            ContactType::Particle { i, j } => {
                // If the querying particle matches the canonical 'i', return as-is
                if query_i == i {
                    return self.clone();
                }

                // Otherwise, query_i must be 'j' (the larger index), so return the flipped version
                let mut flipped = self.clone();
                flipped.normal = -self.normal;
                flipped.rel_vel = -self.rel_vel;
                flipped.tangential_disp = -self.tangential_disp;
                flipped.r_i = self.r_j;
                flipped.r_j = self.r_i;
                flipped.contact_type = ContactType::Particle { i: j, j: i };
                flipped
            }
            ContactType::Plane { .. } => {
                // Planes only have one particle side, so they don't need flipping
                self.clone()
            }
        }
    }
}

impl Default for Contact{
    fn default()-> Self{
        Self { 
            overlap: 1e-5, 
            tangential_disp: DVec3::ZERO,
            normal: DVec3::new(0.0, 0.0, 1.0), 
            r_i: DVec3::ZERO, 
            r_j: DVec3::ZERO, 
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

    
    pub fn analyse_particle_contact(
        &self,
        i: usize,
        j: usize,
        particles: &ParticleVec,
        settings: &SimulationSettings,
    ) -> Option<Contact> {
        //Check whether particles are collision ptypes
        if !(settings.collision_mask[particles.ptype[i]] || settings.collision_mask[particles.ptype[j]]){
            return None;
        }

        // Always sort indices (smaller index first)
        let sorted_i = std::cmp::min(i, j);
        let sorted_j = std::cmp::max(i, j);
        let canonical_type = ContactType::Particle{i:sorted_i, j:sorted_j};

        let entry = self.states.entry(canonical_type);

        match entry {
            dashmap::mapref::entry::Entry::Vacant(vacant) => {
                // Case 1: Does not exist -> Brand new contact
                let contact = compute_particle_contact(sorted_i, sorted_j, particles, settings)?;               

                let saved = contact.clone();
                vacant.insert(saved);

                // Return adjusted for whichever particle (i) asked for it
                Some(contact.get_contact(i))
            }
            dashmap::mapref::entry::Entry::Occupied(mut existing) => {
                let contact_ref = existing.get_mut();

                if !contact_ref.is_active {
                    // Case 2: Exists, but is_active is false -> First update in this loop iteration
                    let mut contact = compute_particle_contact(sorted_i, sorted_j, particles, settings)?;

                    let old_tangential_disp = contact_ref.tangential_disp;
                    let delta_tangential = contact.v_tang() * settings.dt;
                    
                    contact.tangential_disp = old_tangential_disp + delta_tangential;
                    contact.is_active = true;

                    *contact_ref = contact.clone();

                    Some(contact.get_contact(i))
                } else {
                    // Case 3: Exists and is_active is true -> Already computed this step!
                    // Do NOT recalculate. Just pull the stored canonical record and flip it for particle i.
                    Some(contact_ref.get_contact(i))
                }
            }
        }
    }

    pub fn analyse_surface_contact(
        &self,
        i: usize,
        particles: &ParticleVec,
        object: &ObjectSpec,
        settings: &SimulationSettings,
    ) -> Option<Contact> {
        //Check if particle is a collision ptype
        if !settings.collision_mask[particles.ptype[i]] {
            return None;
        }

        let contact_type = ContactType::Plane{i, plane_id: object.id()};
        let entry = self.states.entry(contact_type);

        match entry {
            dashmap::mapref::entry::Entry::Vacant(vacant) => {
                // Case 1: Does not exist -> Brand new contact
                let contact = compute_surface_contact(i, particles, object)?;               

                let saved = contact.clone();
                vacant.insert(saved);

                // Return adjusted for whichever particle (i) asked for it
                Some(contact.get_contact(i))
            }
            dashmap::mapref::entry::Entry::Occupied(mut existing) => {
                    let contact_ref = existing.get_mut();
                    // Case 2: Exists
                    let mut contact = compute_surface_contact(i, particles, object)?;

                    let old_tangential_disp = contact_ref.tangential_disp;
                    let delta_tangential = contact.v_tang() * settings.dt;
                    
                    contact.tangential_disp = old_tangential_disp + delta_tangential;
                    contact.is_active = true;

                    *contact_ref = contact.clone();

                    Some(contact.get_contact(i))
                }
            }
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
                //Prune Contact
                on_separation(contact_type, state);
                false
            } else {
                //Keep Contact
                state.is_active = false; // reset for the next step
                true 
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
    /// * `i`, `j` - Indices of the interacting particles. i must be < j
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
        
        if i>j{
            panic!("i must be less than j");
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

            let contact_type = ContactType::Particle{i, j};

            let contact = Contact {
                overlap,
                tangential_disp: DVec3::ZERO,
                normal,
                r_i,
                r_j,
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


    ///------------------------------------------------------------------------------
    /// compute_surface_contact
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
        particles: &ParticleVec,
        surface: &S,
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
                r_i,
                r_j: DVec3::ZERO,
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
