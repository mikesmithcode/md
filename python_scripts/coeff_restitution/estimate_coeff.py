import numpy as np

# --- Parameters ---
e = 0.7            # Coefficient of restitution (0.7 is fairly bouncy)
r = 0.005         # 0.5mm radius
Y = 1e9            # 1 GPa (Hard plastic/stiff rubber)
density = 1000     # 1000 kg/m^3

# --- 1. Calculate Physical Mass ---
m_particle = (4/3) * np.pi * density * (r**3)

# --- 2. Calculate Effective Mass (for a 2-body collision) ---
m_eff = (m_particle * m_particle) / (m_particle + m_particle) # i.e., m/2

# --- 3. Stiffness (Linear Approximation) ---
# k is proportional to Y * r
k = 0.133 * Y * r

# --- 4. Damping (Analytical solution for Spring-Dashpot) ---
ln_e = np.log(e)
# Corrected damping formula
c = 2 * np.sqrt(m_eff * k) * (np.abs(ln_e) / np.sqrt(np.pi**2 + ln_e**2))

# --- 5. Timestep Stability ---
# Natural period T = 2 * pi * sqrt(m_eff / k)
# Half-period (collision duration) = pi * sqrt(m_eff / k)
t_collision = np.pi * np.sqrt(m_eff / k)

# Safety factor: Aim for at least 20-50 steps per collision
dt_safe = t_collision / 20

cutoff = 2.0 * r
skin = 0.2 * cutoff

print(f"--- Simulation Parameters ---")
print(f"Particle Mass:     {m_particle:.4e} kg")
print(f"Stiffness (k):     {k:.2f} N/m")
print(f"Damping (c):       {c:.6f} Ns/m")
print(f"Collision Time:    {t_collision:.4e} s")
print(f"Recommended dt:    {dt_safe:.4e} s")
print(f"Cutoff should be : {cutoff}")
print(f"skin should be : {skin}")
