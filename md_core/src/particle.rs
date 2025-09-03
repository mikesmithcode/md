// md_sim/src/lib.rs
use glam::f64::DVec3;
use three_d::*;

///Particle defines a particle and allows you to update the positions
#[derive(Debug, Clone, PartialEq)] 
pub struct Particle {
    pub id: usize,
    pub position: DVec3,  
    pub velocity: DVec3, 
    pub color: Srgba,          
    pub radius: f64,           
}

impl Particle {
    pub fn new(id: usize, position: DVec3, velocity: DVec3, color: Srgba, radius: f64) -> Self {
        Particle { id, position, velocity, color, radius }
    }

    pub fn update(&mut self, dt:f64){
        self.position += self.velocity * dt;

}
}
