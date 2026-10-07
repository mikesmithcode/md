# Tangential Friction Models in Discrete Element Method (DEM) Simulations

In Discrete Element Method (DEM) simulations, modeling forces during particle-particle or particle-surface interactions requires handling both normal (collision and repulsion) and tangential (friction and shear) components. While normal forces determine bounce-back and indentation, tangential forces govern shear resistance, rotational torques, energy dissipation during sliding, and microscopic adhesion/stick-slip behavior.

This document explores two distinct tangential interaction models implemented in our DEM architecture:
1. **The Viscous Damping Friction Model** (`friction_viscous_damping`)
2. **The Cundall-Strack History-Dependent Spring-Dashpot Model** (`friction_cundall_strack`)

---

## 1. The Viscous Damping Friction Model

### Physical Picture
The viscous damping friction model treats tangential resistance purely as a velocity-dependent drag force opposing relative tangential sliding motion, bounded by a Coulomb friction limit. It assumes that there is no accumulation of elastic shear displacement (no "memory" of past positions) while particles are in contact. Instead, energy dissipation happens instantaneously based on the relative tangential velocity vector ($\mathbf{v}_{\text{tang}}$) between the contacting bodies.

### Key Assumptions
* **No Shear Memory:** The contact does not track past tangential displacements or micro-displacements. The force resets to zero the instant relative tangential velocity ceases ($\mathbf{v}_{\text{tang}} = 0$).
* **Velocity-Proportional Dissipation:** The tangential force scales linearly with the magnitude of the relative tangential velocity via a constant damping coefficient ($c_t$).
* **Coulomb Yield Criterion:** The resulting viscous shear force cannot exceed the maximum static/kinetic friction threshold dictated by the normal force magnitude ($F_n$) and the friction coefficient ($\mu$):
  $$\|\mathbf{F}_t\| \le \mu F_n$$

### Code Implementation
```rust
#[inline]
pub fn friction_viscous_damping(
    f_normal_mag: f64, 
    contact: &Contact, 
    model: &FrictionViscous
) -> DVec3 {
    let v_tang = contact.rel_vel - contact.rel_vel.dot(contact.normal) * contact.normal;

    if v_tang.length_squared() > 1e-18 {
        let c_t = model.damping_coeff;
        
        let f_t_ideal = v_tang * -c_t;
        let limit = model.mu * f_normal_mag;
        let f_t_mag_sq = f_t_ideal.length_squared();

        if f_t_mag_sq > limit * limit {
            f_t_ideal * (limit / f_t_mag_sq.sqrt())
        } else {
            f_t_ideal
        }
    } else {
        DVec3::ZERO
    }
}
```

---

## 2. The Cundall-Strack History-Dependent Model

### Physical Picture
The Cundall-Strack model (the gold standard in classical DEM) introduces an **elastic spring-dashpot in parallel** combined with a history-dependent tangential displacement accumulator. When two bodies first touch, a virtual tangential contact point is established. As the bodies slide or rotate relative to each other, the tangential shear displacement ($\mathbf{u}_t$) accumulates over time:
$$\mathbf{u}_t^{(t+\Delta t)} = \mathbf{u}_t^{(t)} + \mathbf{v}_{\text{tang}} \Delta t$$

This accumulated displacement creates an elastic restoring force (like a microscopic spring being stretched), alongside a viscous dashpot damping term to dissipate energy. If the resulting trial force exceeds the Coulomb yield envelope ($\mu F_n$), the contact slips, and the stored displacement history is back-corrected so that it doesn't falsely over-accumulate.

### Key Assumptions
* **Shear Memory (Elastic Pre-sliding Behavior):** Unlike pure viscous damping, contacts exhibit elastic compliance before macroscopic sliding occurs, capturing realistic microscopic stick behavior under cyclic loading or vibration.
* **History Tracking:** Requires external state management via a `ContactManager` keyed by contact entity pairs `(i, j)`.
* **Sliding Back-Correction:** When the Coulomb limit is breached, the stored displacement is adjusted dynamically to match the maximum allowed frictional force:
  $$\mathbf{u}_t^{\text{new}} = -\frac{\mathbf{F}_t^{\text{capped}}}{k_t}$$

### Code Implementation
```rust
pub fn friction_cundall_strack(
    f_normal_mag: f64,
    contact: &Contact,
    model: &FrictionCundallStrack,
    contact_manager: &ContactManager,
    i: usize,
    j: usize,
    dt: f64,
) -> DVec3 {
    let pair = (i, j);

    let v_tang = contact.rel_vel - contact.rel_vel.dot(contact.normal) * contact.normal;

    let delta_tangential = v_tang * dt;
    contact_manager.check_or_add(pair, delta_tangential);

    let full_tang_disp = contact_manager.states.get(&pair).unwrap().tangential_disp;

    let k_t = (2.0 / 3.0) * model.tang_stiffness;
    let c_t = 0.5 * model.tang_damping;

    let mut f_t_ideal = -k_t * full_tang_disp - c_t * v_tang;

    let limit = model.mu * f_normal_mag;
    let f_t_mag_sq = f_t_ideal.length_squared();

    if f_t_mag_sq > limit * limit && f_t_mag_sq > 1e-18 {
        let f_t_mag = f_t_mag_sq.sqrt();
        f_t_ideal = f_t_ideal * (limit / f_t_mag);

        if let Some(mut state) = contact_manager.states.get_mut(&pair) {
            state.tangential_disp = -f_t_ideal / k_t;
        }
    }

    f_t_ideal
}
```

---

## 3. Choosing Suitable Parameter Values

Selecting appropriate values for $\mu$, stiffness, and damping coefficients depends heavily on the physical system being modeled (e.g., dry granular powders, rocks, beads, or geotechnical soils).

### A. Friction Coefficient ($\mu$)
* **Definition:** Represents the macroscopic angle of friction or sliding resistance between surfaces.
* **Typical Ranges:**
  * **Glass Beads / Polished Steel:** $0.1$ to $0.2$
  * **Dry Sand / Gravel / Rock:** $0.4$ to $0.7$
  * **Cohesive or Rough Powders:** $0.8$ to $1.2+$
* **Selection Tip:** Start by matching experimental angle of repose measurements for bulk material tests.

### B. Tangential Stiffness ($k_t$ / `tang_stiffness`)
* **Theoretical Context:** In Hertzian contact theory, tangential shear stiffness is coupled to normal stiffness. Mindlin's classic formulation suggests:
  $$k_t \approx \frac{2}{3} k_n$$
  where $k_n$ is the normal contact stiffness.
* **Selection Tip:** If using independent parameters, ensure $k_t$ remains proportional to normal stiffness to maintain numerical stability. Setting $k_t$ excessively high forces the simulation time step ($\Delta t$) to be extremely small to satisfy Courant stability conditions ($\Delta t < 2 \sqrt{m_{\text{eff}} / k_t}$).

### C. Tangential Damping ($c_t$ / `tang_damping` or `damping_coeff`)
* **Theoretical Context:** Controls how quickly shear oscillations decay during contact. Often configured as a fraction of normal damping:
  $$c_t \approx 0.5 \text{ to } 0.8 \times c_n$$
* **Selection Tip:** High damping coefficients eliminate artificial bouncing and acoustic ringing quickly, producing stable quasi-static packings. Low damping preserves elastic rebound energy in high-speed impact dynamics (e.g., ball milling or chute flows).
