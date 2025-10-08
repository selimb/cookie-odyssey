use std::{path::Path, process::Stdio};

use anyhow::Context;

/// Transcodes a video file to a universal format (H.264 video codec, AAC audio codec).
/// We do this because user-uploaded videos can be in weird formats, e.g. some Android
/// phones use H.265 and don't include the video length in the metadata.
pub fn transcode_video(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    // Example implementation using ffmpeg command line tool
    let output = std::process::Command::new("ffmpeg")
        .args([
            "-i",
            &input_path.to_string_lossy(),
            // Use H.264 video codec (the universal codec).
            "-c:v",
            "libx264",
            // Constant Rate Factor (lower is better quality, 18-23 visually lossless).
            "-crf",
            "23",
            // Controls encoding speed/compression.
            // Since this runs in the background, we can afford to be slow.
            "-preset",
            "veryslow",
            // Audio codec.
            "-c:a",
            "aac",
            // Audio bitrate.
            "-b:a",
            "128k",
            // Overwrite output file if it exists.
            "-y",
            &output_path.to_string_lossy(),
        ])
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .output()
        .context("Failed to run ffmpeg")?;

    let status = output.status;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!(
            "ffmpeg exited with status: {}\n# stdout: {}\n# stderr: {}",
            status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANDROID_VIDEO_PATH: &str = "test_data/video_transcoding/android_video.mp4";

    #[test]
    fn test_transcode_video() {
        // Add your test implementation here
        let input_path = Path::new(ANDROID_VIDEO_PATH);
        assert!(input_path.is_file());

        let output_path = Path::new("tmp").join(input_path.file_name().unwrap());

        transcode_video(input_path, &output_path).unwrap();
    }
}
