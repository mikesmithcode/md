# Simulation Descriptions

## abp

Active Brownian Particles in a periodic box.

For each particle i we generate some random numbers. We then calculate the noise scale.
The variance of the random displacement in time dt is 2*Dt*dt but we will multiply this by 
dt when we calculate the displacement in motion part. Friction F is gamma * v. The noise must be (2*gamma**2 * Dt/dt)**0.5

The particle collisions are handled using the WCA which is a truncated lennards-Jones potential that stops at the minimum of the potential.

## ball_surface

Inelastic particle bouncing on a surface. The surface is defined mathematically as a horizontal plane at a height h.

## coeff

One ball dropping on top of a fixed ball. The collision is inelastic.

## silo

Crystalline array of particles is dropped into a closed silo. The collisions are inelastic.

## simple_bouncing_balls

One big ball dropping onto a surface constructed from small particles.

## 2D rotation

Simplest case of a particle with rotation and friction.
