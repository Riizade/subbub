use anyhow::{anyhow, Result};
use srtlib::Subtitles;
use std::{hash, path::Path, process::Command};

use crate::core::data::{pretty_cmd, pretty_output};

use super::data::{hash_subtitles, SyncTool, TMP_DIRECTORY};

pub fn sync(reference: &Subtitles, unsynced: &Subtitles, method: &SyncTool) -> Result<Subtitles> {
    match method {
        SyncTool::FFSUBSYNC => sync_ffsubsync(reference, unsynced),
    }
}

fn sync_ffsubsync(reference: &Subtitles, unsynced: &Subtitles) -> Result<Subtitles> {
    let reference_hash = hash_subtitles(reference);
    let reference_file = TMP_DIRECTORY
        .get()
        .unwrap()
        .join(format!("sync_ref_{reference_hash}.srt"));
    reference.write_to_file(&reference_file, None)?;

    let unsynced_hash = hash_subtitles(unsynced);
    let unsynced_file = TMP_DIRECTORY
        .get()
        .unwrap()
        .join(format!("unsynced_{unsynced_hash}.srt"));
    unsynced.write_to_file(&unsynced_file, None)?;

    let tmp_file = TMP_DIRECTORY
        .get()
        .unwrap()
        .join(format!("sync_out_{reference_hash}_{unsynced_hash}.srt"));

    let mut command = Command::new("ffsubsync");
    command
        .arg(reference_file.as_os_str())
        .arg("-i")
        .arg(unsynced_file.as_os_str())
        .arg("-o")
        .arg(tmp_file.as_os_str());
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
    let subtitles = Subtitles::parse_from_file(tmp_file, None)?;

    Ok(subtitles)
}
