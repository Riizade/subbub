use anyhow::Result;
use srtlib::Subtitles;
use std::{hash, path::Path, process::Command};

use super::data::{hash_subtitles, SubtitleSource, TMP_DIRECTORY};

pub fn sync(reference: &Subtitles, unsynced: &Subtitles) -> Result<Subtitles> {
    let reference_hash = hash_subtitles(reference);
    let reference_file = TMP_DIRECTORY
        .get()
        .unwrap()
        .join(format!("sync_reference_{reference_hash}.srt"));
    reference.write_to_file(&reference_file, None)?;

    let unsynced_hash = hash_subtitles(unsynced);
    let unsynced_file = TMP_DIRECTORY
        .get()
        .unwrap()
        .join(format!("sync_unsynced_{unsynced_hash}.srt"));
    unsynced.write_to_file(&unsynced_file, None)?;

    let tmp_file = TMP_DIRECTORY
        .get()
        .unwrap()
        .join(format!("sync_output_{reference_hash}_{unsynced_hash}.srt"));

    let output = Command::new("ffsubsync")
        .arg(reference_file.as_os_str())
        .arg("-i")
        .arg(unsynced_file.as_os_str())
        .arg("-o")
        .arg(tmp_file.as_os_str())
        .output()?;

    let subtitles = Subtitles::parse_from_file(tmp_file, None)?;

    Ok(subtitles)
}
