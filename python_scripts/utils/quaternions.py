import numpy as np
import math

def quaternion(axis: tuple[float, float, float], theta: float) -> tuple[float, float, float, float]:
    """
    Converts a 3D axis and a rotation angle (in radians) into a normalised unit quaternion.
    
    Returns:
        tuple: (x, y, z, w)
    """
    # 1. Ensure the input axis vector is normalised to unit length
    ax, ay, az = axis
    magnitude = math.sqrt(ax**2 + ay**2 + az**2)
    
    # Handle the edge case of a zero vector passed as an axis
    if magnitude == 0:
        return (0.0, 0.0, 0.0, 1.0) # Return the identity quaternion
        
    ux = ax / magnitude
    uy = ay / magnitude
    uz = az / magnitude

    # 2. Calculate half-angle trigonometric values
    half_theta = theta / 2.0
    sin_half = math.sin(half_theta)
    cos_half = math.cos(half_theta)

    # 3. Formulate the quaternion components
    x = ux * sin_half
    y = uy * sin_half
    z = uz * sin_half
    w = cos_half

    return (x, y, z, w)

def theta_from_quaternion_xz(df):
    """
    Calculates (cos(theta), sin(theta)) from quaternion arrays or scalars.
    Assumes a rotation around the Y-axis acting on local forward vector [1, 0, 0].
    """
    # Force inputs into numpy arrays to support both scalars and series safely
    qy = np.array(df['qy'])
    qw = np.array(df['qw'])
    
    cos_theta = qw**2 - qy**2
    sin_theta = -2.0 * qy * qw
    
    return (cos_theta, sin_theta)



colours = {
    0: 'red',
    1: 'blue',
    2: 'green',
    3: 'cyan',
    4: 'magenta',
    5: 'yellow'
}


