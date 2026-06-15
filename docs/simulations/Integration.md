# Integration algorithms for molecular dynamics simulations

## Verlet integration

The Verlet integration algorithm is a numerical method used to integrate Newton's equations of motion in molecular dynamics simulations. It is based on the Taylor expansion of the position and velocity of particles, and it provides a simple and efficient way to update the positions and velocities of particles over time.

The basic idea behind the Verlet integration algorithm is to use the positions and velocities of particles at the current time step to calculate their positions and velocities at the next time step. The algorithm can be expressed mathematically as follows:

```rust   
    x(t + Δt) = 2x(t) - x(t - Δt) + a(t)Δt^2
    v(t + Δt) = (x(t + Δt) - x(t - Δt)) / (2Δt)
```
Where:
- `x(t)` is the position of the particle at time `t`
- `v(t)` is the velocity of the particle at time `t`    



The Damping coefficient should be ~ 0.1-0.5 of C where C is the critical damping coefficient, which can be calculated as:

```rust
    C = 2 * sqrt(m * k)
```
Where m is the mass of the particle and k is the spring constant of the system. The critical damping coefficient represents the point at which the system transitions from underdamped to overdamped behavior. Choosing a damping coefficient in the range of 0.1-0.5 of C allows for effective energy dissipation.

If density 1000kgm^-3, particle radius 0.1m, and spring constant 100N/m, the critical damping coefficient should be about 0.2kg/s

