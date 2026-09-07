## testnormalcollisions

Two small particles moving in -z with same velocity. One collides with much larger stationary particle. The other collides with a stationary rectangle surface. They should bounce back in almost identical ways since a large sphere looks like a surface. 

```bash
run -c testnormalcollisions
run testnormalcollisions
```

This simulation is very slow because we write a file every few timesteps. Stop after one collision. Then run analysis:

```bash
run -a testnormalcollisions
```

The two curves showing the ball bouncing on ball and surface should sit pretty much on top of one another. Check the coefficient of restitution gives $|v_i| \sim |v_f|$

## testnormalcollisions2

Two balls moving in vz = -5 direction. Collide with one ball that has a) vz=0 and another with b) vz = +5. All balls have same mass.

a) Moving ball hits stationary ball. Coeff restitution = 0.7

$$m u_1 + m u_2 = m v_1 + m v_2$$
$$m(v) + m(0) = m v_1 + m v_2$$
$$v_1 + v_2 = v \quad \text{(1)}$$

$$e = \frac{v_2 - v_1}{u_1 - u_2}$$
$$0.7 = \frac{v_2 - v_1}{v - 0}$$
$$v_2 - v_1 = 0.7v \quad \text{(2)}$$

$$\begin{cases} v_1 + v_2 = v \\ -v_1 + v_2 = 0.7v \end{cases}$$

$$(v_1 + v_2) + (-v_1 + v_2) = v + 0.7v$$
$$2v_2 = 1.7v$$
$$v_2 = 0.85v$$

$$(v_1 + v_2) - (-v_1 + v_2) = v - 0.7v$$
$$2v_1 = 0.3v$$
$$v_1 = 0.15v$$



b) Moving ball hits ball with equal and opposite velocity

$$m u_1 + m u_2 = m v_1 + m v_2$$
$$m(v) + m(-v) = m v_1 + m v_2$$
$$v_1 + v_2 = 0 \quad \text{(1)}$$

$$e = \frac{v_2 - v_1}{u_1 - u_2}$$
$$0.7 = \frac{v_2 - v_1}{v - (-v)}$$
$$0.7 = \frac{v_2 - v_1}{2v}$$
$$v_2 - v_1 = 1.4v \quad \text{(2)}$$

$$\begin{cases} v_1 + v_2 = 0 \\ -v_1 + v_2 = 1.4v \end{cases}$$

$$v_2 = -v_1 \implies (-v_1) - v_1 = 1.4v$$
$$-2v_1 = 1.4v$$
$$v_1 = -0.7v$$

$$v_2 = -(-0.7v) = 0.7v$$

## testrolling

A solid sphere rolling down a tilted slope with a vertical height drop of $\Delta h$. 

The initial gravitational potential energy at the top is converted into translational and rotational kinetic energy:

$$m g \Delta h = K_{\text{trans}} + K_{\text{rot}}$$

$$m g \Delta h = \frac{1}{2} m v^2 + \frac{1}{2} I \omega^2$$

For a uniform solid sphere of mass $m$ and radius $r$, the moment of inertia is:

$$I = \frac{2}{5} m r^2$$

The rolling-without-slipping condition relates angular velocity $\omega$ to linear velocity $v$:

$$\omega = \frac{v}{r}$$

Substituting $I$ and $\omega$ into the energy equation:

$$m g \Delta h = \frac{1}{2} m v^2 + \frac{1}{2} \left( \frac{2}{5} m r^2 \right) \left( \frac{v}{r} \right)^2$$

$$m g \Delta h = \frac{1}{2} m v^2 + \frac{1}{5} m v^2$$

$$m g \Delta h = \left( \frac{1}{2} + \frac{1}{5} \right) m v^2$$

$$m g \Delta h = \frac{7}{10} m v^2$$

Cancelling $m$ and solving for $v$:

$$v^2 = \frac{10}{7} g \Delta h$$

$$v = \sqrt{\frac{10}{7} g \Delta h}$$