## Summary of simulations

1. silo

This is a 2d simulation using particles with no friction, rotation etc just inelastic collisions. The hopper is made of particles with sloped boundaries made of particles with ptype=1. The bottom of the silo is horizontal made of particles with ptype=2. A square grid of particles (ptype=0) is initially suspended above the hopper and then falls under gravity. One could perhaps in future then remove the bottom particles and watch things flow out.

2. simple_bouncing_balls

This is a 2d simulation in which we have 1 large ball dropped onto a horizontal surface constructed of small balls which is oscillating sinusoidally in the vertical direction with small amplitude. Collisions are inelastic. No friction or rotation.

3. ball_surface

This is a single ball falling onto a static surface where the surface is just a plane defined mathematically.

3. coeff

One ball bouncing on top of another stationary ball. Can be used to estimate coefficient of restitution in a collision.

4. abp

An implementation of active brownian particles. Overlap is controlled with a Weeks-Chandler-Andersen potential (Lennard-Jones with a cutoff at the minimum). The particles have a swim speed v0, a translational noise controlled by Dt and a gaussian angular noise. The angular diffusion Dr is related to the translation Dt by Dr = 3.0*Dt/(4.0 * r**2). This should produce MIPs like behaviour for high densities, low angular noise / high swim speed.


