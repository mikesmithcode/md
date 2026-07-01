//! particle.rs
//!
//! This module provides the key definitions of a particle:
//! - position
//! - velocity
//! - radius
//! 
//!  *IMPORTANT*
//!  It uses the soa_derive macro. This means that we appear to define a Vec of Particle structs
//! but what actually happens is that a struct of Vecs is created. e.g if my Particle struct had 
//! See docs for details: <https://docs.rs/soa_derive/latest/soa_derive/>
//! We do this because iterating over a Vec of positions is really fast compared to taking a whole particle one
//! at a time into the cache memory. However, its much easier to think about structs. This gives us the best of both worlds.
//! 

// md_sim/src/lib.rs
use glam::f64::{DVec3,DQuat};
use three_d::*;
use soa_derive::StructOfArray;


///Particle defines a particle object which defines key properties: position, velocity etc
#[derive(Debug, Clone, PartialEq, StructOfArray)]
#[soa_derive(Debug, PartialEq)]
pub struct Particle {
    pub id: usize,
    pub molecule_id: usize,
    pub ptype: usize,
    pub position: DVec3,  
    pub rel_pos: DVec3,
    pub velocity: DVec3,          
    pub orientation: DQuat,
    pub omega: DVec3,
    pub radius: f64, 
    pub mass: f64,  
    pub charge: f64,
    pub color: Srgba, 
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
    /// * `orientation` - Initial orientation, set to 0,0,0 if not needed.
    /// * `omega` - Initial angular velocity, set to 0,0,0 if not needed.
    /// * `radius` - The physical radius of the spherical particle.
    /// * `density` - The mass per unit volume.
    /// * `charge` - charge
    /// * `color` - The RGBA colour used for rendering.
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
        color: Srgba
    ) -> Self {
        // Calculate mass: m = volume * density
        let volume = (4.0 / 3.0) * std::f64::consts::PI * radius.powi(3);
        let mass = volume * density;
        let ref_pos = DVec3::ZERO;

        

        Particle { 
            id, 
            molecule_id,
            ptype, 
            position,
            rel_pos, 
            velocity, 
            orientation,
            omega,
            radius, 
            mass, 
            charge,
            color,
            ref_pos
        }
    }
}


