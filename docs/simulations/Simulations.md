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

5. 2d_rotation

Two particles. One falling onto the other. The bottom one is set as immobile. The particles can rotate and interact with loss and friction. The top particle falls under gravity. ptypes -> Moving particle = 0, charge associated with moving particle = 2, static particle = 1, charge associated with static particle = 3. Uses the SolidFriction Model. "active_ptypes" specified in the config are those particles which respond to forces on them. Specifying that a particle ptype is immobile in update_ptype_no_forces means that any forces calculated on these particles are set to zero. This means they don't move in response to forces.


## Understanding config files

- ptype is used to indicate different types of particles
- interaction_ptypes contains a list of interaction pairs:
    [[0,0],[0,1],[0,2],[1,2]] means 0 is active and will respond to forces from other 0's, 1 and 2 and 1 is active and will respond to forces from 2 but as listed will not respond to forces from 0s or 1s but because it is first it will respond to single body forces such as gravity. 2 is never first so it is an inactive particle and will be static and do nothing except exert forces on others.
