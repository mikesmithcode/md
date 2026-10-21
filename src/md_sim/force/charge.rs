
use crate::md_sim::ParticleVec;

/// qmax is the maximum magnitude of charge on a patch
/// dq is the amount of charge to add +ve or negative.
pub fn add_charge(i:usize, particles: &mut ParticleVec, dq: f64, qmax: f64){
    let mut q = particles.charge[i];
    let mut colour = particles.colour[i];
    
    //positive charges red, negative blue, neutral white
    if (q>0.0) || ((q == 0.0) & (dq > 0.0)){
        q=(q+dq).min(qmax);
        particles.charge[i] = q;
        colour.r=255;
        colour.b = (255.0*(1.0-q/qmax)) as u8;
        colour.g = (255.0*(1.0-q/qmax)) as u8;
        particles.colour[i] = colour;
    }else{
        //qmax is a magnitude and therefore always +ve
        q=(q+dq).max(-qmax);
        particles.charge[i] = q;
        colour.b=255;
        colour.r = (255.0*(1.0+q/qmax)) as u8;
        colour.g = (255.0*(1.0+q/qmax)) as u8;
        particles.colour[i] = colour;
        
    }
    println!("add_charge: q {}, qmax {}", q, qmax);
    println!("color {:?}", colour);
    
}
