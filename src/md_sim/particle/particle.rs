//! particle.rs
//!
//! This module provides the key definitions of a particle:
//! - position
//! - velocity
//! - radius
//! 
//!  *IMPORTANT*
//!  It uses the soa_derive macro. This means that we appear to define a Vec of Particle structs
//! but what actually happens is that a struct of Vecs is created. e.g if my Particle struct had .x and .y for each particle
//! and then these were pushed to a vec. The macro takes this and creates a vec of x and vec of y and attaches them as fields to 
//! a ParticleVec struct.
//! 
//! See docs for details: <https://docs.rs/soa_derive/latest/soa_derive/>
//! 
//! We do this because iterating over a Vec of positions is really fast compared to taking a whole particle one
//! at a time into the cache memory. However, its much easier to think about structs. This gives us the best of both worlds.


// md_sim/src/lib.rs
use glam::f64::{DVec3,DQuat};
use three_d::*;
use soa_derive::StructOfArray;

use crate::md_viz::SceneSettings;
use crate::md_sim::MoleculeMap;


///Particle defines a particle object which defines key properties: position, velocity etc
/// 
/// id is unique to every particle
/// molecule_id - all particles that belong to the same superstructure have the same molecule_id. 
/// ptype - defines different types of particles. Mostly used to target different interactions between different types of particles
/// position - global coordinates of particle
/// rel_pos - local coordinate of particle in a molecule 
/// velocity - global velocity of particle
/// orientation - a quaternion  
#[derive(Debug, Clone, PartialEq, StructOfArray)]
#[soa_derive(Debug, PartialEq)]
pub struct Particle {
    pub id: usize,
    pub molecule_id: usize,
    pub ptype: usize,
    pub position: DVec3,  
    pub rel_pos: DVec3,
    pub display_pos: DVec3,
    pub velocity: DVec3,          
    pub orientation: DQuat,
    pub omega: DVec3,
    pub radius: f64, 
    pub mass: f64,  
    pub charge: f64,
    pub colour: Srgba, 
    pub visible: bool,
    // Verlet lists tracker fields
    pub ref_pos: DVec3,    
}

impl Particle {
    /// Initialises a new spherical particle and calculates its mass.
    ///
    /// The mass is derived from the volume of a sphere ($V = \frac{4}{3}\pi r^3$) 
    /// multiplied by the provided density.
    ///
    /// # Arguments
    ///
    /// * `id` - A unique identifier for the particle.
    /// * `molecule_id` - id shared by all particles in a molecule. If isolated particle set to 
    /// * `ptype` - The category ID (used for filtering or specific behaviours).
    /// * `position` - Initial coordinates in the simulation box.
    /// * `rel_pos` - This is the position relative to COM of particle.
    /// * `velocity` - Initial velocity vector.
    /// * `orientation` - Initial orientation, set to 1.0,0.0,0.0,0.0 if not needed.
    /// * `omega` - Initial angular velocity, set to 0,0,0 if not needed.
    /// * `radius` - The physical radius of the spherical particle.
    /// * `density` - The mass per unit volume.
    /// * `charge` - charge
    /// * `colour` - The Colour enum used for rendering. Determines whether hidden, transparent or Opaque and holds the Srgba colour.
    /// * `visible` - whether the particle should be rendered
    /// * `ref_pos` - used by the CellGrid verlet list building algorithm to track how far each particle has been displaced since the last rebuild
    /// 
    pub fn new(
        id: usize,
        molecule_id: usize, 
        ptype: usize, 
        position: DVec3,
        rel_pos: DVec3, 
        velocity: DVec3, 
        orientation: DQuat,
        omega: DVec3,
        radius: f64, 
        density: f64, 
        charge: f64,
        colour: Srgba,
        visible: bool
    ) -> Self {
        // Calculate mass: m = volume * density
        let volume = (4.0 / 3.0) * std::f64::consts::PI * radius.powi(3);
        let mass = volume * density;
        let ref_pos = DVec3::ZERO;
        let display_pos = DVec3::ZERO;

        Particle { 
            id, 
            molecule_id,
            ptype, 
            position,
            rel_pos, 
            display_pos,
            velocity, 
            orientation,
            omega,
            radius, 
            mass, 
            charge,
            colour,
            visible,
            ref_pos
        }
    }
}


impl Default for Particle{
    fn default()-> Self{
        Self { 
            id:0, 
            molecule_id: 0, 
            ptype: 0, 
            position: DVec3::new(1.0,1.0,1.0), 
            rel_pos: DVec3::ZERO,
            display_pos: DVec3::ZERO,
            velocity: DVec3::ZERO,
            orientation: DQuat::IDENTITY, //(w,x,y,z) = (1.0,0.0,0.0,0.0)
            omega: DVec3::ZERO, 
            radius: 0.5, 
            mass: 1.0, 
            charge: 0.0, 
            colour: Srgba { r: 255, g: 255, b: 255, a: 200 }, 
            visible: true, 
            ref_pos: DVec3::ZERO }
    }
}

impl ParticleVec{
    pub fn update_display_pos(&mut self, molecule_map: &MoleculeMap, patch_scale: f64) {
        for mol in molecule_map.values() {
            // Find the lead particle (ptype == 0) in the molecule
            let lead_idx = match mol.pids.iter().find(|&&idx| self.ptype[idx] == 0) {
                Some(&idx) => idx,
                None => mol.pids[0], // Fallback to first if not explicitly found
            };

            let lead_pos = self.position[lead_idx];
            let core_radius = self.radius[lead_idx];

            // Lead particle core display position matches its physical position
            self.display_pos[lead_idx] = lead_pos;

            // Iterate through all other particles (patches) in the molecule
            for &idx in &mol.pids {
                if idx == lead_idx {
                    continue;
                }

                // Calculate direction vector from core center to the patch's physical position
                let particle_pos = self.position[idx];
                let diff = particle_pos - lead_pos;
                let dist = diff.length();

                if dist > 1e-6 {
                    let dir = diff / dist;
                    
                    // Position the display sphere outward by a fraction/scale defined in SceneSettings
                    // e.g., patch_offset_scale could be close to 1.0 (or slightly larger than core radius)
                    let offset_distance = core_radius * patch_scale;
                    self.display_pos[idx] = lead_pos + dir * offset_distance;
                } else {
                    self.display_pos[idx] = lead_pos;
                }
            }
        }
    }
}