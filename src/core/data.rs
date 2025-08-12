use crate::core::ffmpeg;
use anyhow::Result;
use clap::ValueEnum;
use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};
use srtlib::Subtitles;
use std::{
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

pub enum VideoSource {
    File(PathBuf),
    Directory(PathBuf),
}

impl VideoSource {
    pub fn to_videos(&self) -> Result<Vec<PathBuf>> {
        match self {
            VideoSource::File(pathbuf) => {
                if pathbuf.exists() && is_video_file(pathbuf) {
                    Ok(vec![pathbuf.clone()])
                } else {
                    Err(anyhow::anyhow!(
                        "File does not exist or is not a valid video file: {}",
                        pathbuf.to_string_lossy()
                    ))
                }
            }
            VideoSource::Directory(pathbuf) => {
                if pathbuf.exists() && pathbuf.is_dir() {
                    Ok(list_video_files(pathbuf))
                } else {
                    Err(anyhow::anyhow!(
                        "Directory does not exist: {}",
                        pathbuf.to_string_lossy()
                    ))
                }
            }
        }
    }
}

impl TryFrom<&str> for VideoSource {
    type Error = anyhow::Error;
    fn try_from(s: &str) -> Result<Self> {
        let pathbuf = PathBuf::from(s);
        return VideoSource::try_from(pathbuf);
    }
}

impl TryFrom<PathBuf> for VideoSource {
    type Error = anyhow::Error;
    fn try_from(pathbuf: PathBuf) -> Result<Self> {
        if pathbuf.is_dir() {
            Ok(VideoSource::Directory(pathbuf))
        } else if pathbuf.is_file() && is_video_file(&pathbuf) {
            Ok(VideoSource::File(pathbuf))
        } else {
            Err(anyhow::Error::msg(format!(
                "Invalid video source: {}",
                pathbuf.to_string_lossy()
            )))
        }
    }
}

pub enum SubtitleSource {
    File(PathBuf),
    VideoTrack {
        video_file: PathBuf,
        subtitle_track: u32,
    },
    Directory(PathBuf),
}

impl TryFrom<&str> for SubtitleSource {
    type Error = anyhow::Error;
    fn try_from(s: &str) -> Result<Self> {
        // if the string contains a ":" character, we parse it as a video track
        if s.contains(':') {
            let parts: Vec<&str> = s.split(':').collect();
            if parts.len() != 2 {
                panic!("Invalid video track format: {}", s);
            }
            let video_file = PathBuf::from(parts[0]);
            let subtitle_track: u32 = parts[1].parse().expect("Invalid subtitle track number");
            return Ok(SubtitleSource::VideoTrack {
                video_file,
                subtitle_track,
            });
        }
        // otherwise, we assume it's a file path
        else {
            let pathbuf = PathBuf::from(s);
            if !pathbuf.exists() {
                return Err(anyhow::Error::msg(format!(
                    "File or directory does not exist: {}",
                    pathbuf.to_string_lossy()
                )));
            } else if pathbuf.exists() && pathbuf.is_file() && is_subtitle_file(&pathbuf) {
                // if the file exists and is a subtitle file, we return it as a file source
                return Ok(SubtitleSource::File(pathbuf));
            } else if pathbuf.is_dir() {
                return Ok(SubtitleSource::Directory(pathbuf));
            } else if pathbuf.is_file() {
                return Ok(SubtitleSource::File(pathbuf));
            } else {
                return Err(anyhow::Error::msg(format!(
                    "Invalid subtitle source: {}; unknown error occurred",
                    pathbuf.to_string_lossy()
                )));
            }
        }
    }
}

impl From<SubtitleSource> for String {
    fn from(source: SubtitleSource) -> Self {
        match source {
            SubtitleSource::File(pathbuf) => pathbuf.to_string_lossy().to_string(),
            SubtitleSource::Directory(pathbuf) => pathbuf.to_string_lossy().to_string(),
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

pub struct DiskSubtitles {
    pub path: PathBuf,
    pub subtitles: Subtitles,
}

impl DiskSubtitles {
    pub fn subtitles_string(&self) -> String {
        self.subtitles.to_string()
    }
}

impl PartialEq for DiskSubtitles {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl PartialOrd for DiskSubtitles {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.path.partial_cmp(&other.path)
    }
}

impl Eq for DiskSubtitles {}

impl Ord for DiskSubtitles {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.path.cmp(&other.path)
    }
}

impl SubtitleSource {
    pub fn to_subtitles(&self) -> Result<Vec<DiskSubtitles>> {
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
                Ok(vec![DiskSubtitles {
                    path: pathbuf.to_owned(),
                    subtitles,
                }])
            }
            SubtitleSource::VideoTrack {
                video_file,
                subtitle_track,
            } => {
                let s = ffmpeg::extract_subtitles(video_file, *subtitle_track)?;
                Ok(vec![DiskSubtitles {
                    path: video_file.to_owned(),
                    subtitles: s,
                }])
            }
            SubtitleSource::Directory(pathbuf) => {
                let mut subtitles = Vec::new();
                for entry in pathbuf.read_dir()? {
                    let entry = entry?;
                    let path = entry.path();
                    if is_subtitle_file(&path) {
                        let s = Subtitles::parse_from_file(&path, None)?;
                        subtitles.push(DiskSubtitles {
                            path: path.to_owned(),
                            subtitles: s,
                        });
                    }
                }
                Ok(subtitles)
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
