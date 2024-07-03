use anyhow::{anyhow, Context, Result};
use srtlib::Subtitles;
use std::{path::Path, process::Command};

use crate::core::data::{pretty_cmd, pretty_output, TMP_DIRECTORY};

use super::data::hash_string;

pub fn add_subtitles_track(
    video_file: &Path,
    subtitles_file: &Path,
    track_number: u32,
    language_code: &str,
    output_path: &Path,
) -> Result<()> {
    let mut command = Command::new("mkvmerge");
    command
        .arg("-o") // specify the output path
        .arg(output_path)
        .arg(video_file)// input the video file
        .arg("--language") // add the language code
        .arg(format!("{track_number}:{language_code}"))
        .arg("--track-name") // name the track
        .arg(format!("{track_number}:{language_code}"))
        .arg(subtitles_file)// input the subtitles file
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
