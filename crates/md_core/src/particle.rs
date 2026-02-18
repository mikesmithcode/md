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

///Particle defines a particle object which defines key properties: position, velocity etc
#[derive(Debug, Clone, PartialEq)] 
pub struct Particle {
    pub id: usize,
    pub position: DVec3,  
    pub velocity: DVec3, 
    pub color: Srgba,          
    pub radius: f64,           
}

impl Particle {
    ///Create a new particle
    pub fn new(id: usize, position: DVec3, velocity: DVec3, color: Srgba, radius: f64) -> Self {
        Particle { id, position, velocity, color, radius }
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
        let radius = 0.5;

        let particle = Particle::new(id, position, velocity, color, radius);

        assert_eq!(particle.id, id);
        assert_eq!(particle.position, position);
        assert_eq!(particle.velocity, velocity);
        assert_eq!(particle.color, color);
        assert_eq!(particle.radius, radius);
    }
}
