use md::md_viz::video::assemble_pngs_to_mp4;
use std::path::Path;

fn main(){

    assemble_pngs_to_mp4(Path::new("output")).expect("Video writing failed");

}
