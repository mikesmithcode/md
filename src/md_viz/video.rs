//! video.rs
use std::path::PathBuf;
use std::process::{Command, Stdio, Child};
use std::io::Write;
use std::fs;
use crate::md_viz::scene::SceneSetup;

pub struct VideoExporter {
    process: Child,
    video_path: PathBuf,
}

impl VideoExporter {
    pub fn new(video_path: &PathBuf, scene_settings: &SceneSetup) -> Result<Self, Box<dyn std::error::Error>> {        
        if let Some(parent) = video_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let width = scene_settings.window_size.0;
        let height = scene_settings.window_size.1;
        let fps = scene_settings.vid_fps;

        let process = Command::new("ffmpeg")
            .args([
                "-y", 
                "-f", "rawvideo", 
                "-pix_fmt", "rgba", 
                "-s", &format!("{}x{}", width, height),
                "-r", &format!("{}", fps),
                "-i", "-", 
                "-c:v", "libx264", 
                "-preset", "ultrafast", 
                "-crf", "18", 
                "-pix_fmt", "yuv420p", 
            ])
            .arg(&video_path)
            .stdin(Stdio::piped())
            .spawn()?;

        Ok(Self { process, video_path: video_path.clone()})
    }

    pub fn write_frame(&mut self, pixels: &[ [u8; 4] ]) -> Result<(), std::io::Error> {
        let stdin = self.process.stdin.as_mut().expect("Failed to access ffmpeg stdin");
        let raw_bytes: &[u8] = bytemuck::cast_slice(pixels);
        stdin.write_all(raw_bytes)
    }

    pub fn close(mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Closing stdin signals EOF to ffmpeg to finish the file
        std::mem::drop(self.process.stdin.take());
        let status = self.process.wait()?;
        
        if status.success() {
            println!("Video file {} successfully finalised.", self.video_path.to_string_lossy());
        } else {
            eprintln!("FFmpeg exited with an error.");
        }
        Ok(())
    }
}
