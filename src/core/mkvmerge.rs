use anyhow::{anyhow, Result};

use std::{path::Path, process::Command};

use crate::core::data::{pretty_cmd, pretty_output, SubtitleTrack};

fn confirm_mkvmerge() -> Result<()> {
    let mut command = Command::new("mkvmerge");
    command.arg("--version");
    command
        .output()
        .expect("mkvmerge is not present; install mkvtoolnix to fix");
    Ok(())
}

pub fn add_subtitles_track(
    video_file: &Path,
    subtitles_file: SubtitleTrack,
    output_path: &Path,
) -> Result<()> {
    add_subtitles_tracks(video_file, vec![subtitles_file], output_path)
}

pub fn add_subtitles_tracks(
    video_file: &Path,
    subtitles_files: Vec<SubtitleTrack>,
    output_path: &Path,
) -> Result<()> {
    confirm_mkvmerge()?;
    let mut command = Command::new("mkvmerge");

    for subs in subtitles_files {
        // determine the track name for the subtitles
        let actual_track_name = match &subs.title {
            Some(t) => t.clone(),
            None => subs
                .path
                .file_stem()
                .expect("subtitles file had no file name")
                .to_str()
                .expect("could not convert OsStr to str")
                .to_owned(),
        };
        // provide track naming arguments + subtitle filepath
        command
            .arg("--track-name") // name the track
            .arg(format!("0:{actual_track_name}"))
            .arg(subs.path.as_os_str()); // input the subtitles file

        // provide language code arguments if present
        if let Some(code) = &subs.language_code {
            command
                .arg("--language") // add the language code
                .arg(format!("0:{code}"));
        }
    }

    command
        .arg("-o") // specify the output path
        .arg(output_path)
        .arg(video_file)// input the video file
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
