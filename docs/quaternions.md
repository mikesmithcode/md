# Quaternions in Molecular Dynamics

Quaternions provide an efficient, numerically stable way to represent the 3D rotation of rigid bodies.

## Mathematical Definition

When rotating an object by an angle $\theta$ around a normalised 3D axis vector $\mathbf{u} = (u_x, u_y, u_z)$, the components of the unit quaternion $\mathbf{q} = (x, y, z, w)$ are defined as:

$$x = u_x \sin\left(\frac{\theta}{2}\right)$$
$$y = u_y \sin\left(\frac{\theta}{2}\right)$$
$$z = u_z \sin\left(\frac{\theta}{2}\right)$$
$$w = \cos\left(\frac{\theta}{2}\right)$$

A valid rotation quaternion must always be normalised to maintain unit length, satisfying the condition:

$$x^2 + y^2 + z^2 + w^2 = 1$$

## Frame Transformations (Local to Global)

A rigid molecule has a fixed physical shape defined in its own **local reference frame**, where its constituent atoms or surface patches do not move relative to one another. As the molecule moves and rotates throughout the simulation, its orientation changes with respect to the **global laboratory frame**.

The quaternion $\mathbf{q}$ acts as a transformation operator that maps vectors from the local frame to the global frame. 

$$\mathbf{v}_{\text{global}} = \mathbf{q} \mathbf{v}_{\text{local}} \mathbf{q}^{-1}$$

In Rust code using the `glam` library, this sandwich product is handled automatically behind the scenes using the multiplication operator:

```rust
// Transforms a local direction vector into the global simulation space
let global_vector = particles.orientation[i] * local_vector;
```

## The Identity State
When a particle has experienced no rotation relative to the global axes ($\theta = 0$), the half-angle trigonometric functions evaluate to:
* $\sin(0) = 0$
* $\cos(0) = 1$

This yields the **identity quaternion**:
$$\mathbf{q}_{\text{identity}} = (0, 0, 0, 1)$$

Multiplying any local vector by the identity quaternion returns the exact same vector unchanged in the global frame.

## Worked Example: A 90-Degree Rotation around the Y-Axis

Consider a 2D simulation running in the X-Z plane. We want to rotate a rigid sphere by 90 degrees ($\theta = \frac{\pi}{2}$ radians) counter-clockwise around the vertical Y-axis.

### 1. Finding the Axis and Angle
* **Normalized Axis Vector:** $\mathbf{u} = (0, 1, 0)$
* **Rotation Angle:** $\theta = 90^\circ = \frac{\pi}{2}$
* **Half-Angle Components:** $\frac{\theta}{2} = 45^\circ = \frac{\pi}{4}$

### 2. Calculating the Quaternion Components
Using the half-angle trigonometric identities ($\sin(45^\circ) = \frac{1}{\sqrt{2}} \approx 0.7071$ and $\cos(45^\circ) = \frac{1}{\sqrt{2}} \approx 0.7071$):

$$x = u_x \sin\left(\frac{\pi}{4}\right) = 0 \cdot 0.7071 = 0$$
$$y = u_y \sin\left(\frac{\pi}{4}\right) = 1 \cdot 0.7071 = 0.7071$$
$$z = u_z \sin\left(\frac{\pi}{4}\right) = 0 \cdot 0.7071 = 0$$
$$w = \cos\left(\frac{\pi}{4}\right) = 0.7071$$

The resulting unit quaternion is:
$$\mathbf{q} = (0, \, 0.7071, \, 0, \, 0.7071)$$

### 3. Verification of Unit Length
We verify that the sum of the squares equals 1:
$$x^2 + y^2 + z^2 + w^2 = 0^2 + (0.7071)^2 + 0^2 + (0.7071)^2 = 0.5 + 0.5 = 1.0$$

### 4. Vector Transformation Example
If the molecule has a local patch pointing straight down its **positive X-axis** ($\mathbf{v}_{\text{local}} = [1, 0, 0]$), multiplying it by this quaternion will rotate it 90 degrees onto the **negative Z-axis**:

$$\mathbf{v}_{\text{global}} = \mathbf{q} \times [1, 0, 0] = [0, 0, -1]$$

In Rust, this is computed instantaneously:
```rust
let orientation = glam::DQuat::from_axis_angle(glam::DVec3::Y, std::f64::consts::FRAC_PI_2);
let local_patch = glam::DVec3::X; // [1.0, 0.0, 0.0]

let global_patch = orientation * local_patch; // Results in [0.0, 0.0, -1.0]
```

In our polars dataframes which define the state of the simulation we have position (x,y,z), rel_position (rel_x, rel_y, rel_z), orientation quaternion (qx,qy,qz,qw). 

The `position` always defines the absolute position in the global coordinate space of a particle. This could be part of a composite particle. In a composite particle the centre of mass of the particle will be at $$\mathbf{R}_{com} = \frac{1}{M} \sum_{i=1}^{n} m_i \mathbf{r}_i$$.

The `rel_pos` is in the local frame of the molecule. So each particle in a composite particle has a relative position compared with the centre of mass of the molecule. No matter how the particle rotates in the global frame this stays the same.

The `orientation` is the quaternion which transforms between local and global positions.

### 5. Integration of Quaternions in Molecular Dynamics Simulations

In molecular dynamics simulations, quaternions are often used to update the orientation of rigid bodies over time. The orientation quaternion can be updated based on angular velocity and time step using quaternion algebra.

To do this we need to calculate the torque acting on the body. This consists of the forces acting on constituent particles about the centre of mass, the sum of torques acting on each constituent particle.

$$\mathbf{\tau}_{total} = \sum_{i=1}^{n} \mathbf{\tau}_i + \sum_{i=1}^{n} \mathbf{r}_i \times \mathbf{F}_i$$

```rust
    for &idx in &mol.pids {
            let r = particles.position[idx] - com_pos;
            total_torque += torques[idx] + r.cross(forces[idx]);
        }
```
Since we are using a half-step verlet integration scheme, we need to update the orientation quaternion using the angular velocity at the half-step. This takes place in the Motion Trait function `update_motion`.

The Quaternion `particle.orientation` specifies the orientation of the molecule in the global frame. This is the same for all constituent particles of the molecule. We use this to calculate the rotation matrix ($R$). This matrix can take a local vector defining where each particle in a molecule is relative to the centre of mass and transform it into the global frame.

The moment of inertia in the global frame may not be the same as in the local frame. This is because it can be rotating around an arbitrary axis. 

$$I_{global} = R I_{local} R^T$$

```rust
        let rot_mat = DMat3::from_quat(particles.orientation[lead_idx]);
        let i_global = rot_mat * mol.inertia * rot_mat.transpose();
```

The angular acceleration is calculated using the total torque and the $I_global$.

```rust
        let omega = particles.omega[lead_idx];
        let gyroscopic = omega.cross(i_global * omega);
        let alpha = i_global.inverse() * (total_torque - gyroscopic);
```
We then use the angular acceleration to update the angular velocity $\omega$ at the half-step and then use this to update the orientation quaternion.

```rust
        let new_omega = omega + (alpha * half_dt);

        // Update Orientation and COM Position
        let new_com_pos = com_pos + (new_com_vel * dt);
        let delta_q = DQuat::from_scaled_axis(new_omega * dt);
        let new_orientation = (delta_q * particles.orientation[lead_idx]).normalize();

        // Update every particle's state
        let rot_mat_new = DMat3::from_quat(new_orientation);
        for &idx in &mol.pids {
            // Update individual velocity: v_i = v_com + (omega x r_global)
            let r_global = rot_mat_new * particles.rel_pos[idx];
            particles.velocity[idx] = new_com_vel + new_omega.cross(r_global);
            
            // Update individual position
            particles.position[idx] = new_com_pos + r_global;
            
            // Sync orientation and omega (if stored per-particle)
            particles.orientation[idx] = new_orientation;
            particles.omega[idx] = new_omega;
        }
```

The simulation loop then updates forces and torques based on the new positions and orientations. We then complete the final half-step integration of the angular velocity using these new values of the force and torque. This is in the Motion trait function `correct_motion`.
