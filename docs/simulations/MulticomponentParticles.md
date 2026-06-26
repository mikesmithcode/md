# Multi-component particles

## rigid bodies

In these simulations we assume that each body is composed of multiple particles combined into a "molecule" ie a superstructure. All particles move and rotate together.

In the Forces trait we calculate all the forces $f_{i}$ and torques $\tau_{i}$ on particles individually. In the Motion trait we then work out the centre of mass of the molecule as all forces and torques act around this point. The force on the molecule which results in translation is the vector sum of the individual forces acting through the point $R_{COM}$. In the case of the torques we have two terms:

$$\tau_{total} = \sum_{i}\tau_{i} + \sum_{i}(r_{i}-R_{COM}) \times f_{i}$$

ie the sum of the individual torques plus the torque about the centre of mass of the forces on the particles.

### Verlet integration scheme.

In verlet we update the velocity and the omega first integrating half a timestep. Then we integrate the positions / orientations by a full timestep using the new values of velocity and omega. Then we calculate the new forces and torques. Finally, we apply a correction to the velocity and omega by integrating a further half-step.

### Update the velocity 

We calculate the total mass, centre of mass and mean velocity of the molecule. We then sum the total force and calculate  torques. We then simply use F=ma to calculate the acceleration and modify the velocity (half timestep):

$$v = a*dt/2$$

## Applying the rotation

See the detailed notes on [quaternions](quaternions.md). 




