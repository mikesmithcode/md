use std::path::{Path, PathBuf};
use std::process::Command;
use std::io::{self, Write};
use std::fs;



/// The default framerate for the output video.
const DEFAULT_FRAMERATE: &str = "25"; 

/// Assembles all `.png` images in the given directory into a single MP4 video 
/// using FFmpeg.
/// 
/// The function relies on the `ffmpeg` executable being available in the system's PATH.
/// 
/// Arguments:
/// - `dir_path`: The directory containing the `.png` sequence files.
/// 
/// Returns:
/// A `Result` indicating success or an `io::Error`.
pub fn assemble_pngs_to_mp4(dir_path: &Path) -> Result<(), io::Error> {
    
    println!("Starting video assembly process for directory: {:?}", dir_path);

    // Find and sort all PNG files.
    let png_files = find_and_sort_pngs(dir_path)?;

    // Determine the output path based on the first file.
    let first_file_name = png_files[0].file_stem()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Could not determine file name."))?;
        
    let output_file = dir_path.join(format!("{}.mp4", first_file_name.to_string_lossy()));
    
    // Create the temporary list file for FFmpeg's concat demuxer.
    let list_file_path = dir_path.join("ffmpeg_list.txt");
    create_ffmpeg_list_file(&list_file_path, &png_files)?;

    println!("Output video will be saved to: {:?}", output_file);
    
    // 4. Construct and execute the FFmpeg command.
    let result = execute_ffmpeg(
        &list_file_path, 
        &output_file
    );

    // 5. Cleanup: Always attempt to delete the temporary list file.
    if let Err(e) = fs::remove_file(&list_file_path) {
        eprintln!("Warning: Failed to remove temporary list file {:?}: {}", list_file_path, e);
    }
    
    // Check the result of the FFmpeg execution
    match result {
        Ok(_) => {
            println!("\n✅ Successfully assembled video: {:?}", output_file);
            Ok(())
        }
        Err(e) => {
            eprintln!("\n❌ FFmpeg execution failed: {}", e);
            Err(e)
        }
    }
}

/// Scans the directory, filters for `.png` files, and sorts them lexicographically.
fn find_and_sort_pngs(dir_path: &Path) -> Result<Vec<PathBuf>, io::Error> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir_path)?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "png") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    // Sort files by name to ensure correct frame order (e.g., frame_001.png, frame_002.png)
    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    
    Ok(files)
}

/// Creates a temporary text file in the FFmpeg concat demuxer format.
/// Example line: `file 'frame_001.png'`
fn create_ffmpeg_list_file(list_path: &Path, png_files: &[PathBuf]) -> Result<(), io::Error> {
    let mut file = fs::File::create(list_path)?;
    
    for path in png_files {
        // Use file_name() and to_string_lossy() to get the string representation 
        // of the filename, ensuring paths are correctly quoted for FFmpeg.
        let file_name = path.file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name encountered."))?
            .to_string_lossy();
            
        // FFmpeg concat demuxer format: file 'filename'
        writeln!(file, "file '{}'", file_name)?;
    }
    
    Ok(())
}

/// Executes the FFmpeg command to combine the images listed in the input file.
fn execute_ffmpeg(list_file_path: &Path, output_file_path: &Path) -> Result<(), io::Error> {
    
    let status = Command::new("ffmpeg")
        // -f concat: Use the concat demuxer
        .arg("-f").arg("concat")
        // -safe 0: Allows relative paths in the list file (necessary for files in the same dir)
        .arg("-safe").arg("0")
        // -i: Input file (the list file we created)
        .arg("-i").arg(list_file_path)
        // -r: Set the framerate (e.g., 25 frames per second)
        .arg("-r").arg(DEFAULT_FRAMERATE)
        // -c:v: Video codec (H.264, standard for mp4)
        .arg("-c:v").arg("libx264")
        // -pix_fmt yuv420p: Ensure compatibility with all media players (required for MP4/H.264)
        .arg("-pix_fmt").arg("yuv420p")
        // Output file
        .arg(output_file_path)
        // Execute command and capture status
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("FFmpeg failed with exit code: {:?}", status.code())
        ))
    }
}
