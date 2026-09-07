use glam::DVec3;

#[inline(always)]
pub (crate) fn compute_contact_force_and_torque(
    overlap: f64,
    normal: DVec3,
    r_contact: DVec3,
    rel_vel: DVec3,
    eff_stiffness: f64,
    eff_damping: f64,
    mu: f64,
) -> (DVec3, DVec3) {
    let normal_vel = rel_vel.dot(normal);

    // Elastic and viscous damping normal forces
    let f_elastic = eff_stiffness * overlap;
    let f_damping = eff_damping * normal_vel;
    let f_normal_mag = (f_elastic - f_damping).max(0.0);
    let f_normal_vec = normal * f_normal_mag;

    // Tangential friction force
    let v_tang = rel_vel - normal_vel * normal;
    let mut f_friction_vec = DVec3::ZERO;

    if v_tang.length_squared() > 1e-18 {
        let f_t_ideal = v_tang * -eff_damping;
        let limit = mu * f_normal_mag;
        let f_t_mag_sq = f_t_ideal.length_squared();

        f_friction_vec = if f_t_mag_sq > limit * limit {
            f_t_ideal * (limit / f_t_mag_sq.sqrt())
        } else {
            f_t_ideal
        };
    }

    let force = f_normal_vec + f_friction_vec;
    let torque = r_contact.cross(f_friction_vec);

    (force, torque)
}