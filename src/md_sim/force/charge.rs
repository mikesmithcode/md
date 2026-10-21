
use crate::md_sim::ParticleVec;

pub fn add_charge(i:usize, particles: &mut ParticleVec, dq: f64, qmax: f64){
    let mut q = particles.charge[i];
    let mut colour = particles.colour[i];
    println!("Adding dq {}, to q {} max qmax {}",dq, q, qmax);
    if dq*qmax < 0.0{
        panic!("add_charge assumes adding negative charge dq to negative qmax or +ve to +ve");
    }
    //positive charges red, negative blue
    if q >= 0.0{
        q=(q+dq).min(qmax);
        particles.charge[i] = q;
        colour.b = (255.0*(1.0-q/qmax)) as u8;
        colour.g = (255.0*(1.0-q/qmax)) as u8;
        particles.colour[i] = colour;
    }else{
        //qmax and dq are negative
        q=(q+dq).max(qmax);
        println!("q capped {}", q);
        particles.charge[i] = q;
        colour.r = (255.0*(1.0-q/qmax)) as u8;
        colour.g = (255.0*(1.0-q/qmax)) as u8;
        println!("rd {}", colour.r);
        particles.colour[i] = colour;
    }
}
