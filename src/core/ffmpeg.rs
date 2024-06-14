// functions that invoke ffmpeg
use anyhow::Result;
use srtlib::Subtitles;
use std::{path::Path, process::Command};

use crate::core::data::TMP_DIRECTORY;

pub fn extract_subtitles(video_file: &Path, subtitle_track: u32) -> Result<Subtitles> {
    let tmp_file = TMP_DIRECTORY.get().unwrap().join(format!(
        "extracted_subs_{0}_{1}.srt",
        video_file.file_stem().unwrap().to_string_lossy(),
        subtitle_track
    ));

    let output = Command::new("ffmpeg")
        .arg("-i") // select the input video
        .arg(video_file.as_os_str())
        .arg("-map") //select the subtitle track
        .arg(format!("0:s:{subtitle_track}"))
        .arg("-c:s") // convert subtitles to srt format
        .arg("srt")
        .arg(tmp_file.as_os_str()) // select the output file
        .output()?;

    let subs = read_subtitles_file(&tmp_file)?;

    Ok(subs)
}

pub fn read_subtitles_file(path: &Path) -> Result<Subtitles> {
    let tmp_file = TMP_DIRECTORY.get().unwrap().join(format!(
        "converted_subs_{0}.srt",
        path.file_stem().unwrap().to_string_lossy()
    ));
    let output = Command::new("ffmpeg")
        .arg("-i") // select input subtitles file
        .arg(path.as_os_str())
        .arg("-c:s") // convert to srt format
        .arg("srt")
        .arg(tmp_file.as_os_str()) // output file
        .output()?;

    let subs = Subtitles::parse_from_file(tmp_file, None)?;

    Ok(subs)
}
