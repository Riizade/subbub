// functions that invoke ffmpeg
use anyhow::Result;
use srtlib::Subtitles;
use std::{path::Path, process::Command};

use crate::core::data::TMP_DIRECTORY;

pub fn extract_subtitles(video_file: &Path, subtitle_track: u32) -> Result<Subtitles> {
    let tmp_file = TMP_DIRECTORY.get().unwrap().join(format!(
        "/extracted_subs_{0}_{1}",
        video_file.to_string_lossy(),
        subtitle_track
    ));

    let output = Command::new("ffmpeg")
        .arg("-i")
        .arg(video_file.as_os_str())
        .arg("-map")
        .arg(format!("0:s:{subtitle_track}"))
        .arg(tmp_file.as_os_str())
        .output()?;

    println!("{output:#?}");

    let string = Subtitles::parse_from_file(tmp_file, None)?;

    Ok(string)
}
