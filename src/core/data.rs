use crate::core::ffmpeg;
use anyhow::Result;
use once_cell::sync::{Lazy, OnceCell};
use srtlib::Subtitles;
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
};

pub static TMP_DIRECTORY: Lazy<OnceCell<PathBuf>> = Lazy::new(|| OnceCell::from(tmp_directory()));
pub const VIDEO_FILE_EXTENSIONS: [&str; 3] = ["mkv", "mp4", "avi"];
pub const SUBTITLES_FILE_EXTENSIONS: [&str; 3] = ["ass", "ssa", "srt"];

fn tmp_directory() -> PathBuf {
    let dir = PathBuf::from("tmp/");

    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .expect(&format!("could not create temporary directory {dir:#?}"));
    }

    dir
}

pub fn list_video_files(directory: &Path) -> Vec<PathBuf> {
    directory
        .read_dir()
        .unwrap()
        .into_iter()
        .flat_map(|entry| {
            let path = entry.unwrap().path();
            if let Some(ext) = path.extension() {
                if VIDEO_FILE_EXTENSIONS.contains(&ext.to_string_lossy().to_string().as_str()) {
                    return Some(path);
                }
            }
            None
        })
        .collect()
}

pub fn list_subtitles_files(directory: &Path) -> Vec<PathBuf> {
    directory
        .read_dir()
        .unwrap()
        .into_iter()
        .flat_map(|entry| {
            let path = entry.unwrap().path();
            if let Some(ext) = path.extension() {
                if SUBTITLES_FILE_EXTENSIONS.contains(&ext.to_string_lossy().to_string().as_str()) {
                    return Some(path);
                }
            }
            None
        })
        .collect()
}

pub enum SubtitleSource {
    File(PathBuf),
    VideoTrack {
        video_file: PathBuf,
        subtitle_track: u32,
    },
}

impl SubtitleSource {
    pub fn to_subtitles(&self) -> Result<Subtitles> {
        match self {
            SubtitleSource::File(pathbuf) => {
                let extension = pathbuf.extension().unwrap();
                let subtitles = if extension == "srt" {
                    // if the subtitles are already srt format, we can read them directly
                    Subtitles::parse_from_file(pathbuf, None)?
                } else {
                    // otherwise, we need to convert the file using ffmpeg first
                    ffmpeg::read_subtitles_file(pathbuf)?
                };
                Ok(subtitles)
            }
            SubtitleSource::VideoTrack {
                video_file,
                subtitle_track,
            } => {
                let s = ffmpeg::extract_subtitles(video_file, *subtitle_track)?;
                Ok(s)
            }
        }
    }
}

pub fn hash_subtitles(subtitles: &Subtitles) -> u64 {
    let s = subtitles.to_string();
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}
