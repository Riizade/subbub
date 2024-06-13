// this file contains the CLI binary for subbub

use std::path::{Path, PathBuf};

use anyhow::anyhow;
use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use srtlib::Subtitles;
use subbub::core::data::SubtitleSource;
use subbub::core::data::{list_subtitles_files, list_video_files, TMP_DIRECTORY};
use subbub::core::ffmpeg;
use subbub::core::merge::merge;
use subbub::core::sync::sync;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// takes a directory of subtitles and matches them to the subtitles in the given video directory
    /// syncs the given subtitles, then outputs them in the video directory
    FillWithReference {
        /// the directory of subtitles to sync
        #[arg(short = 's', long)]
        subtitles_directory: PathBuf,
        /// the directory containing the video files whose subtitles will be used as a reference
        #[arg(short = 'v', long, default_value = "./")]
        videos_directory: PathBuf,
        /// when this flag is present, creates dual subs alongside the single-sub synced versions (using the reference subs and the given subs)
        #[arg(short = 'd', long)]
        create_dual_subs: bool,
        /// the suffix to give to the subtitles (subs will print as VIDEO_NAME.SUFFIX.srt)
        #[arg(short = 'x', long)]
        suffix: String,
        /// the index of the subtitle track to use as a reference from the video files
        #[arg(short = 't', long, default_value = "0")]
        subtitles_track: u32,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::FillWithReference {
            subtitles_directory,
            videos_directory,
            create_dual_subs,
            suffix,
            subtitles_track,
        } => fill_with_reference(
            subtitles_directory,
            videos_directory,
            *create_dual_subs,
            suffix,
            *subtitles_track,
        ),
    };

    // clean up
    let tmp_dir = TMP_DIRECTORY.get().unwrap();
    if tmp_dir.exists() {
        std::fs::remove_dir_all(TMP_DIRECTORY.get().unwrap())
            .expect("could not remove tmp directory");
    }

    result.expect("command execution failed");
}

fn fill_with_reference(
    subtitles_directory: &Path,
    videos_directory: &Path,
    create_dual_subs: bool,
    suffix: &str,
    subtitles_track: u32,
) -> Result<()> {
    let mut sub_files = list_subtitles_files(subtitles_directory);
    let mut video_files = list_video_files(videos_directory);

    if sub_files.len() != video_files.len() {
        return Err(anyhow!(
            "number of videos and number of subtitles differ, videos: {0}, subs: {1}",
            video_files.len(),
            sub_files.len()
        ));
    }

    sub_files.sort();
    video_files.sort();

    let pairs = sub_files.iter().zip(video_files.iter());

    let mut output_subs: Vec<(Subtitles, PathBuf)> = vec![];

    for (sub_file, video_file) in pairs {
        let file_stem = video_file.file_stem().unwrap();
        let video_subs = ffmpeg::extract_subtitles(video_file, subtitles_track)?;
        let unsynced_subs = SubtitleSource::File(sub_file.clone()).to_subtitles()?;
        let synced_subs = sync(&video_subs, &unsynced_subs)?;
        let synced_subs_output_file =
            PathBuf::from(format!("{0}.{1}.srt", file_stem.to_string_lossy(), suffix));
        output_subs.push((synced_subs.clone(), synced_subs_output_file));

        if create_dual_subs {
            let merged_subs = merge(&video_subs, &synced_subs)?;
            let merged_subs_output_file = PathBuf::from(format!(
                "{0}.dual-{1}.srt",
                file_stem.to_string_lossy(),
                suffix
            ));

            output_subs.push((merged_subs, merged_subs_output_file));
        }
    }

    for (subtitles, output_file) in output_subs.iter() {
        subtitles.write_to_file(output_file, None)?;
    }

    Ok(())
}