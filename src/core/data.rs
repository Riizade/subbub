use crate::core::ffmpeg;
use anyhow::Result;
use once_cell::sync::{Lazy, OnceCell};
use srtlib::Subtitles;
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
};

pub static TMP_DIRECTORY: Lazy<OnceCell<PathBuf>> = Lazy::new(|| OnceCell::from(tmp_directory()));

fn tmp_directory() -> PathBuf {
    let dir = PathBuf::from("tmp/");

    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .expect(&format!("could not create temporary directory {dir:#?}"));
    }

    dir
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
                let subtitles = Subtitles::parse_from_file(pathbuf, None)?;
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
