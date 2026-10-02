use glam::DVec3;
use std::collections::{HashMap, HashSet};

pub struct Contact{
    pub overlap: f64,
    pub normal: DVec3,
    pub r_contact: DVec3,
    pub rel_vel: DVec3,
    pub eff_stiffness: f64,
    pub eff_damping: f64,
    pub mu: f64,
}

pub struct ContactState {
    pub tangential_disp: DVec3,
    pub is_active: bool, // Tracks whether it was touched this step
}

pub struct ContactManager {
    // (i, j) -> accumulated tangential displacement (xi)
    pub states: HashMap<(usize, usize), ContactState>,
}

impl ContactManager {
    pub fn new()->Self{
        Self{
           states: HashMap::new(), 
        }
    }

    pub fn check_or_add(pair: (usize,usize), displacement: DVec3){
        //If new add a ContactState with 0 tangential displacement and is_active = true.
        // If already present calculate new tangential displacement set is_active = true

    }

    pub fn remove_old_contacts(&mut self) {
        // removes anything not active. resets is_active bool to false.   
        self.states.retain(|_, state|{
            let active = state.is_active;
            state.is_active = false;
            active
        } );
    }
}

#[inline]
fn normal_hertzian(contact: &Contact)-> (f64, DVec3){
// Elastic and viscous damping normal forces
    let f_elastic = contact.eff_stiffness * contact.overlap.powf(1.5);
    let f_damping = contact.eff_damping * contact.rel_vel.dot(contact.normal);
    let f_normal_mag = (f_elastic - f_damping).max(0.0);
    
    //f_normal_vec
    (f_normal_mag, contact.normal * f_normal_mag)
}

#[inline]
fn normal_linear(contact: &Contact)-> (f64, DVec3){
    // Elastic and viscous damping normal forces
    let f_elastic = contact.eff_stiffness * contact.overlap;
    let f_damping = contact.eff_damping * contact.rel_vel.dot(contact.normal);
    let f_normal_mag = (f_elastic - f_damping).max(0.0);
    
    //f_normal_vec
    (f_normal_mag, contact.normal * f_normal_mag)
}

fn viscous_tangential_damping_coulomb_limit(f_normal_mag: f64, contact: &Contact)-> DVec3{
    // Tangential friction force
    let v_tang = contact.rel_vel - contact.rel_vel.dot(contact.normal) * contact.normal;
    //let mut f_friction_vec = DVec3::ZERO;

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

pub (crate) fn compute_contact_force_and_torque(
    contact: &Contact,
) -> (DVec3, DVec3) {
    
    let (f_normal_mag ,f_normal_vec) = normal_linear(contact);
    let f_friction_vec = viscous_tangential_damping_coulomb_limit(f_normal_mag, contact);    


    let force = f_normal_vec + f_friction_vec;
    let torque = contact.r_contact.cross(f_friction_vec);

    (force, torque)
}