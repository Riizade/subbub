use anyhow::Result;
use srtlib::Subtitles;
use std::{hash, path::Path, process::Command};

use super::data::{hash_subtitles, SubtitleSource, TMP_DIRECTORY};

pub fn sync(reference: &Subtitles, unsynced: &Subtitles) -> Result<Subtitles> {
    let reference_hash = hash_subtitles(reference);
    let reference_file = TMP_DIRECTORY
        .get()
        .unwrap()
        .join(format!("sync_reference_{reference_hash}"));

    let unsynced_hash = hash_subtitles(unsynced);
    let unsynced_file = TMP_DIRECTORY
        .get()
        .unwrap()
        .join(format!("sync_unsynced_{unsynced_hash}"));

    let tmp_file = TMP_DIRECTORY
        .get()
        .unwrap()
        .join(format!("sync_output_{reference_hash}_{unsynced_hash}"));

    let output = Command::new("ffsubsync")
        .arg(reference_file.as_os_str())
        .arg("-i")
        .arg(unsynced_file.as_os_str())
        .arg("-o")
        .arg(tmp_file.as_os_str())
        .output()?;

    println!("{output:#?}");

    let subtitles = Subtitles::parse_from_file(tmp_file, None)?;

    Ok(subtitles)
}
