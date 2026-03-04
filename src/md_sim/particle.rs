//! particle.rs
//!
//! This module provides the key definitions of a particle:
//! - position
//! - velocity
//! - radius
//! 
//! It could in principle be a composite object with nested structures.
//! 
//! //!
//! # Design
//! - Each particle implements the [`Draw`] trait which defines how to render it

// md_sim/src/lib.rs
use glam::f64::DVec3;
use three_d::*;
use soa_derive::StructOfArray;

///Particle defines a particle object which defines key properties: position, velocity etc
#[derive(Debug, Clone, PartialEq, StructOfArray)]
#[soa_derive(Debug, PartialEq)]
pub struct Particle {
    pub id: usize,
    pub ptype: usize,
    pub position: DVec3,  
    pub velocity: DVec3,          
    pub radius: f64, 
    pub inv_mass: f64,  
    pub color: Srgba,        
}

impl Particle {
    ///Create a new particle
    pub fn new(id: usize, ptype: usize, position: DVec3, velocity: DVec3, radius: f64, density: f64, color: Srgba) -> Self {
        let inv_mass = 1.0/((4.0 / 3.0) * std::f64::consts::PI * radius.powi(3) * density);

        Particle { id, ptype, position, velocity, radius, inv_mass, color}
    }
}



//Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_new() {

        let id = 1;
        let position = DVec3::new(1.0, 2.0, 3.0);
        let velocity = DVec3::new(0.1, 0.2, 0.3);
        let color = Srgba::new(255, 0, 0, 255);
        let radius: f64 = 0.5;
        let density: f64=1.0;
        
        let inv_mass = 1.0/((4.0 / 3.0) * std::f64::consts::PI * radius.powf(3f64) * density);
        let particle = Particle::new(id, position, velocity, radius, density, color);

        assert_eq!(particle.id, id);
        assert_eq!(particle.position, position);
        assert_eq!(particle.velocity, velocity);
        assert_eq!(particle.color, color);
        assert_eq!(particle.radius, radius);
        assert_eq!(particle.inv_mass, inv_mass);
    }
}
