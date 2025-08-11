use crate::core::ffmpeg;
use anyhow::Result;
use clap::ValueEnum;
use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};
use srtlib::Subtitles;
use std::{
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    process::{Command, Output},
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

pub fn is_subtitle_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        if SUBTITLES_FILE_EXTENSIONS.contains(&ext.to_string_lossy().to_string().as_str()) {
            return true;
        }
    }

    false
}

pub fn is_video_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        if VIDEO_FILE_EXTENSIONS.contains(&ext.to_string_lossy().to_string().as_str()) {
            return true;
        }
    }

    false
}

pub fn list_video_files(directory: &Path) -> Vec<PathBuf> {
    directory
        .read_dir()
        .unwrap()
        .into_iter()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            if is_video_file(&path) {
                Some(path)
            } else {
                None
            }
        })
        .collect()
}

pub fn list_subtitles_files(directory: &Path) -> Vec<PathBuf> {
    directory
        .read_dir()
        .unwrap()
        .into_iter()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            if is_subtitle_file(&path) {
                Some(path)
            } else {
                None
            }
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

impl From<String> for SubtitleSource {
    fn from(s: String) -> Self {
        // if the string contains a ":" character, we parse it as a video track
        if s.contains(':') {
            let parts: Vec<&str> = s.split(':').collect();
            if parts.len() != 2 {
                panic!("Invalid video track format: {}", s);
            }
            let video_file = PathBuf::from(parts[0]);
            let subtitle_track: u32 = parts[1].parse().expect("Invalid subtitle track number");
            return SubtitleSource::VideoTrack {
                video_file,
                subtitle_track,
            };
        }
        // otherwise, we assume it's a file path
        else {
            let pathbuf = PathBuf::from(s);
            SubtitleSource::File(pathbuf)
        }
    }
}

impl From<SubtitleSource> for String {
    fn from(source: SubtitleSource) -> Self {
        match source {
            SubtitleSource::File(pathbuf) => pathbuf.to_string_lossy().to_string(),
            SubtitleSource::VideoTrack {
                video_file,
                subtitle_track,
            } => format!("{}:{}", video_file.to_string_lossy(), subtitle_track),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, ValueEnum, Copy)]
#[serde(rename_all = "snake_case")]
pub enum SyncTool {
    #[serde(alias = "ffsubsync")]
    FFSUBSYNC,
}

#[derive(Serialize, Deserialize, Debug, Clone, ValueEnum, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ShiftDirection {
    #[serde(alias = "-")]
    EARLIER,
    #[serde(alias = "+")]
    LATER,
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
    hash_string(&s)
}

pub fn hash_string(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

pub fn pretty_cmd(cmd: &Command) -> String {
    format!(
        "{} {:?}",
        cmd.get_envs()
            .map(|(key, val)| format!("{:?}={:?}", key, val))
            .fold(String::new(), |a, b| a + &b),
        cmd
    )
}

pub fn pretty_output(output: &Output) -> String {
    let separator = "--------------------";
    let s = format!(
        "status: {0}\n{separator}\nstderr:\n{1}\n{separator}\nstdout:\n{2}\n",
        output.status,
        String::from_utf8(output.stderr.clone()).unwrap(),
        String::from_utf8(output.stdout.clone()).unwrap()
    );
    s
}
