// functions that invoke ffmpeg
use anyhow::{anyhow, Context, Result};
use itertools::Itertools;
use srtlib::Subtitles;
use std::{
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

use crate::core::data::{pretty_cmd, pretty_output, TMP_DIRECTORY};

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

    if !output.status.success() {
        return Err(anyhow!(
            "command was not successfully executed:\n{0}\n{1}",
            pretty_cmd(&command),
            pretty_output(&output)
        ));
    }
    log::trace!("{0}", pretty_output(&output));

    log::debug!("reading from temporary file {tmp_file:#?} extracted from video {video_file:#?}:{subtitle_track}");
    let subs = read_subtitles_file(&tmp_file)?;

    Ok(subs)
}

pub fn add_subtitles_track(
    video_file: &Path,
    subtitles_file: &Path,
    track_number: u32,
    language_code: &str,
    output_path: &Path,
) -> Result<()> {
    let mut command = Command::new("ffmpeg");
    command
        .arg("-i") // input the video file
        .arg(video_file)
        .arg("-i") // input the subtitles file
        .arg(subtitles_file)
        .arg("-map") // map both inputs to the output file
        .arg("0")
        .arg("-map")
        .arg("1")
        .arg("-c") // do not re-encode the video
        .arg("copy")
        .arg("-c:s") // set subtitle format
        .arg("srt")
        .arg("-max_interleave_delta") // workaround for a known issue with mkv + subtitles with large gaps, see https://old.reddit.com/r/ffmpeg/comments/1do9azh/difficulty_adding_subtitles_track_to_video/la8bnh8/
        .arg("0")
        .arg(format!("-metadata:s:s:{track_number}")) // set the track number (and also specify that they're subtitles)
        .arg(format!("language={language_code}")) // add the language code
        .arg(output_path) // finally, the output path of the newly created video file
        ;

    log::debug!("{0}", pretty_cmd(&command));
    let output = command.output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "command was not successfully executed:\n{0}\n{1}",
            pretty_cmd(&command),
            pretty_output(&output)
        ));
    }
    log::trace!("{0}", pretty_output(&output));

    Ok(())
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

    if !output.status.success() {
        return Err(anyhow!(
            "command was not successfully executed:\n{0}\n{1}",
            pretty_cmd(&command),
            pretty_output(&output)
        ));
    }
    log::trace!("{0}", pretty_output(&output));

    log::debug!("reading from temporary file {tmp_file:#?} converted from {path:#?}");
    let subs = Subtitles::parse_from_file(tmp_file, None)?;

    Ok(subs)
}

pub fn number_of_subtitle_streams(video_file: &Path) -> Result<u32> {
    let mut command = Command::new("ffprobe");
    command
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("s")
        .arg("-show_entries")
        .arg("stream=index")
        .arg("-of")
        .arg("csv=p=0")
        .arg(video_file.as_os_str());
    log::debug!("{0}", pretty_cmd(&command));
    let output = command.output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "command was not successfully executed:\n{0}\n{1}",
            pretty_cmd(&command),
            pretty_output(&output)
        ));
    }
    log::trace!("{0}", pretty_output(&output));
    let stdout =
        String::from_utf8(output.stdout.clone()).context("could not parse stdout to utf8")?;
    let len = stdout.split('\n').collect_vec().len();
    Ok(len as u32)
}

pub fn convert_to_mkv(video_file: &Path) -> Result<PathBuf> {
    let mut command = Command::new("ffmpeg");
    let output_file = TMP_DIRECTORY.get().unwrap().join(PathBuf::from_str(
        format!("{0}.mkv", video_file.file_stem().unwrap().to_string_lossy()).as_str(),
    )?);
    command
        .arg("-i") // select input video file
        .arg(video_file.as_os_str())
        .arg("-map") // select all streams
        .arg("0")
        .arg("-c") // copy streams
        .arg("copy")
        .arg(&output_file) // output file
        ;
    log::debug!("{0}", pretty_cmd(&command));
    let output = command.output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "command was not successfully executed:\n{0}\n{1}",
            pretty_cmd(&command),
            pretty_output(&output)
        ));
    }
    log::trace!("{0}", pretty_output(&output));

    Ok(output_file)
}
