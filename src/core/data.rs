use std::path::PathBuf;

pub enum SubtitleSource {
    File(PathBuf),
    VideoTrack {
        video_file: PathBuf,
        subtitle_track: u32,
    },
}
