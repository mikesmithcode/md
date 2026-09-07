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
