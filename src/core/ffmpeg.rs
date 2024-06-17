// functions that invoke ffmpeg
use anyhow::Result;
use srtlib::Subtitles;
use std::{path::Path, process::Command};

use crate::core::data::{pretty_cmd, TMP_DIRECTORY};

use super::data::hash_string;

pub fn extract_subtitles(video_file: &Path, subtitle_track: u32) -> Result<Subtitles> {
    let tmp_file = TMP_DIRECTORY.get().unwrap().join(format!(
        "ext_{0}_{1}.srt",
        hash_string(&video_file.file_stem().unwrap().to_string_lossy()),
        subtitle_track
    ));

    let mut command = Command::new("ffmpeg");
    command
        .arg("-i") // select the input video
        .arg(video_file.as_os_str())
        .arg("-map") //select the subtitle track
        .arg(format!("0:s:{subtitle_track}"))
        .arg("-c:s") // convert subtitles to srt format
        .arg("srt")
        .arg(tmp_file.as_os_str()) // select the output file
        ;
    log::debug!("{0}", pretty_cmd(&command));
    let output = command.output()?;
    log::trace!("{output:#?}");

    let subs = read_subtitles_file(&tmp_file)?;

    Ok(subs)
}

pub fn read_subtitles_file(path: &Path) -> Result<Subtitles> {
    let tmp_file = TMP_DIRECTORY.get().unwrap().join(format!(
        "con_{0}.srt",
        hash_string(&path.file_stem().unwrap().to_string_lossy())
    ));

    let mut command = Command::new("ffmpeg");
    command
        .arg("-i") // select input subtitles file
        .arg(path.as_os_str())
        .arg("-c:s") // convert to srt format
        .arg("srt")
        .arg(tmp_file.as_os_str()) // output file
        ;
    log::debug!("{0}", pretty_cmd(&command));
    let output = command.output()?;
    log::trace!("{output:#?}");

    let subs = Subtitles::parse_from_file(tmp_file, None)?;

    Ok(subs)
}
