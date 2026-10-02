use glam::DVec3;
use dashmap::DashMap;
use dashmap::mapref::one::RefMut;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Contact{
    pub overlap: f64,
    pub normal: DVec3,
    pub r_contact: DVec3,
    pub rel_vel: DVec3,
    pub eff_stiffness: f64,
    pub eff_damping: f64,
    pub mu: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContactState {
    pub tangential_disp: DVec3,
    pub is_active: bool, // Tracks whether it was touched this step
}

#[derive(Clone, Debug)]
pub struct ContactManager {
    // (i, j) -> accumulated tangential displacement (xi)
    pub states: DashMap<(usize, usize), ContactState>,
}

impl ContactManager {
    pub fn new()->Self{
        Self{
           states: DashMap::new(), 
        }
    }

    pub fn get_or_create(&self, i: usize, j: usize) -> RefMut<'_, (usize, usize), ContactState> {
        let mut state = self.states.entry((i,j)).or_insert(ContactState {
            tangential_disp: DVec3::ZERO,
            is_active: true,
        });
        state.is_active = true;
        state
    }

    // Change &mut self to &self here too:
    pub fn remove_old_contacts(&self) {
        // DashMap's retain method takes &self because of interior mutability!
        self.states.retain(|_, state| {
            if state.is_active {
                state.is_active = false; // Reset for the next step
                true                     // Keep contact
            } else {
                false                    // Remove expired contact
            }
        });
    }
}

impl Default for ContactManager {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
pub fn normal_hertzian(contact: &Contact)-> DVec3{
// Elastic and viscous damping normal forces
    let f_elastic = contact.eff_stiffness * contact.overlap.powf(1.5);
    let f_damping = contact.eff_damping * contact.rel_vel.dot(contact.normal);
    let f_normal_mag = (f_elastic - f_damping).max(0.0);
    
    //f_normal_vec
    contact.normal * f_normal_mag
}

#[inline]
pub fn normal_linear(contact: &Contact)-> DVec3{
    // Elastic and viscous damping normal forces
    let f_elastic = contact.eff_stiffness * contact.overlap;
    let f_damping = contact.eff_damping * contact.rel_vel.dot(contact.normal);
    let f_normal_mag = (f_elastic - f_damping).max(0.0);
    
    //f_normal_vec
    contact.normal * f_normal_mag
}

pub fn friction_viscous_tangential(f_normal_mag: f64, contact: &Contact)-> DVec3{
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

pub fn friction_cundall_strack(f_normal_vec: DVec3, contact: &Contact, contact_state: &mut ContactState, dt: f64){
    
    let f_coulomb = contact.mu*f_normal_vec.length();

    let normal_vel_mag = contact.rel_vel.dot(contact.normal);
    let v_tangential = contact.rel_vel - contact.normal * normal_vel_mag;

    contact_state.tangential_disp += v_tangential * dt;

    let mut ft = contact_state.tangential_disp * contact.frictional_stiffness;
    if ft.length() > f_coulomb{
        ft = f_coulomb * ft/ft.length();
    }

    ft
}
