//! video.rs
//! 
//! handles all interactions with ffmpeg. It is used to take png output from simulation and assemble into an mp4.

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
    println!("Searching for images in: {:?}", dir_path);

    let dir_path = dir_path.join("imgs");

    println!("Starting video assembly process for directory: {:?}", &dir_path);

    // Find and sort all PNG files.
    let png_files = find_and_sort_pngs(&dir_path)?;
    println!("{:?}", png_files);

    // Determine the output path based on the first file.
    let first_file_name = png_files[0].file_stem()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Could not determine file name."))?;
        
    let output_file = dir_path.join(format!("{}.mp4", first_file_name.to_string_lossy()));
    println!("{:?}", output_file);

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
    
    // Calculate duration based on your 25fps (1.0 / 25.0)
    let duration = 1.0 / DEFAULT_FRAMERATE.parse::<f64>().unwrap_or(25.0);

    for path in png_files {
        let file_name = path.file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name"))?
            .to_string_lossy();
            
        writeln!(file, "file '{}'", file_name)?;
        writeln!(file, "duration {}", duration)?; // Tell FFmpeg how long to show the frame
    }

    // Due to a quirk in FFmpeg concat, the last file should be repeated 
    // without a duration or specified again to ensure the last frame renders.
    if let Some(last_path) = png_files.last() {
        let last_name = last_path.file_name().unwrap().to_string_lossy();
        writeln!(file, "file '{}'", last_name)?;
    }
    
    Ok(())
}

/// Executes the FFmpeg command to combine the images listed in the input file.
fn execute_ffmpeg(list_file_path: &Path, output_file_path: &Path) -> Result<(), io::Error> {
    
    let work_dir = list_file_path.parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not find image directory"))?;
    let list_name = list_file_path.file_name().unwrap();
    let output_name = output_file_path.file_name().unwrap();

    let status = Command::new("ffmpeg")
        .current_dir(work_dir)
        // -f concat: Use the concat demuxer
        .arg("-f").arg("concat")
        // -safe 0: Allows relative paths in the list file (necessary for files in the same dir)
        .arg("-safe").arg("0")
        // -i: Input file (the list file we created)
        .arg("-i").arg(list_name)
        // -r: Set the framerate (e.g., 25 frames per second)
        .arg("-r").arg(DEFAULT_FRAMERATE)
        // -c:v: Video codec (H.264, standard for mp4)
        .arg("-c:v").arg("libx264")
        // -pix_fmt yuv420p: Ensure compatibility with all media players (required for MP4/H.264)
        .arg("-pix_fmt").arg("yuv420p")
        // Output file
        .arg(output_name)
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


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_find_and_sort_pngs() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory that is deleted when 'dir' goes out of scope
        let dir = tempdir()?;
        let dir_path = dir.path();

        // Create files in a jumbled order to test the sorting logic
        // We use different lengths/names to ensure the lexicographical sort works
        let filenames = [
            "img_0002.png",
            "img_0001.png",
            "not_a_png.txt",
            "img_0010.png",
        ];

        for name in filenames {
            File::create(dir_path.join(name))?;
        }

        // Run the function
        let sorted_files = find_and_sort_pngs(dir_path)?;

        // Checks
        // We expect only 3 files (the .txt should be filtered out)
        assert_eq!(sorted_files.len(), 3);

        // Check that they are in the correct lexicographical order
        assert!(sorted_files[0].to_string_lossy().ends_with("img_0001.png"));
        assert!(sorted_files[1].to_string_lossy().ends_with("img_0002.png"));
        assert!(sorted_files[2].to_string_lossy().ends_with("img_0010.png"));

        Ok(())
    }

    #[test]
    fn test_create_ffmpeg_list_file() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let list_path = dir.path().join("test_list.txt");

        let png_files = vec![
            PathBuf::from("frame_001.png"),
            PathBuf::from("frame_002.png"),
            PathBuf::from("frame_003.png"),
        ];

        create_ffmpeg_list_file(&list_path, &png_files)?;

        // Verify file was created
        assert!(list_path.exists());

        // Verify content
        let content = std::fs::read_to_string(&list_path)?;
        assert!(content.contains("file 'frame_001.png'"));
        assert!(content.contains("file 'frame_002.png'"));
        assert!(content.contains("file 'frame_003.png'"));
        assert!(content.contains("duration"));

        Ok(())
    }

    #[test]
    fn test_create_ffmpeg_list_file_empty() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let list_path = dir.path().join("empty_list.txt");
        let png_files: Vec<PathBuf> = vec![];

        create_ffmpeg_list_file(&list_path, &png_files)?;

        assert!(list_path.exists());
        let content = std::fs::read_to_string(&list_path)?;
        // Should be empty or contain minimal content
        assert!(content.is_empty() || content.trim().is_empty());

        Ok(())
    }

    #[test]
    fn test_find_and_sort_pngs_empty_directory() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let result = find_and_sort_pngs(dir.path());

        // Should succeed but return empty vector
        assert!(result.is_ok());
        assert_eq!(result?.len(), 0);

        Ok(())
    }

    #[test]
    fn test_find_and_sort_pngs_no_pngs() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let dir_path = dir.path();

        File::create(dir_path.join("file1.txt"))?;
        File::create(dir_path.join("file2.jpg"))?;

        let sorted_files = find_and_sort_pngs(dir_path)?;

        assert_eq!(sorted_files.len(), 0);

        Ok(())
    }

    #[test]
    fn test_find_and_sort_pngs_numeric_sorting() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let dir_path = dir.path();

        // Test that lexicographical sort handles numbers correctly
        let filenames = ["frame_9.png", "frame_10.png", "frame_100.png"];
        for name in filenames {
            File::create(dir_path.join(name))?;
        }

        let sorted_files = find_and_sort_pngs(dir_path)?;

        assert_eq!(sorted_files.len(), 3);
        // Lexicographical sort: "frame_10" < "frame_100" < "frame_9"
        assert!(sorted_files[0].to_string_lossy().ends_with("frame_10.png"));
        assert!(sorted_files[1].to_string_lossy().ends_with("frame_100.png"));
        assert!(sorted_files[2].to_string_lossy().ends_with("frame_9.png"));

        Ok(())
    }
}
