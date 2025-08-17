use anyhow::{anyhow, Result};

use std::{path::Path, process::Command};

use crate::core::data::{pretty_cmd, pretty_output};

pub fn add_subtitles_track(
    video_file: &Path,
    subtitles_file: &Path,
    language_code: Option<&str>,
    track_name: &str,
    output_path: &Path,
) -> Result<()> {
    let mut command = Command::new("mkvmerge");
    if let Some(code) = language_code {
        command
            .arg("--language") // add the language code
            .arg(format!("0:{code}"));
    }
    command
        .arg("-o") // specify the output path
        .arg(output_path)
        .arg(video_file)// input the video file
        .arg("--track-name") // name the track
        .arg(format!("0:{track_name}"))
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
