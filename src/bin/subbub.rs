// this file contains the CLI binary for subbub

use std::path::{Path, PathBuf};

use anyhow::anyhow;
use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use srtlib::Subtitles;
use subbub::core::data::SubtitleSource;
use subbub::core::data::SyncTool;
use subbub::core::data::{list_subtitles_files, list_video_files, TMP_DIRECTORY};
use subbub::core::ffmpeg;
use subbub::core::ffmpeg::read_subtitles_file;
use subbub::core::merge::merge;
use subbub::core::modify;
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
        #[arg(short = 'm', long, default_value = "ffsubsync")]
        sync_method: SyncTool,
    },
    /// converts the given subtitle file (or directory) to srt
    ConvertSubtitles {
        #[arg(short = 'i', long)]
        input: PathBuf,
        #[arg(short = 'o', long)]
        output: PathBuf,
    },
    /// strips html from the given subtitle file or directory
    StripHtml {
        #[arg(short = 'i', long)]
        input: PathBuf,
        #[arg(short = 'o', long)]
        output: PathBuf,
    },
    /// command for testing
    #[cfg(debug_assertions)]
    Debug,
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
            sync_method,
        } => fill_with_reference(
            subtitles_directory,
            videos_directory,
            *create_dual_subs,
            suffix,
            *subtitles_track,
            sync_method,
        ),
        Commands::ConvertSubtitles { input, output } => convert_subtitles(input, output),
        Commands::StripHtml { input, output } => strip_html(input, output),
        #[cfg(debug_assertions)]
        Commands::Debug => debug(),
    };

    // clean up
    let tmp_dir = TMP_DIRECTORY.get().unwrap();
    if tmp_dir.exists() {
        std::fs::remove_dir_all(TMP_DIRECTORY.get().unwrap())
            .expect("could not remove tmp directory");
    }

    result.expect("command execution failed");
}

/// takes in an input path and output path for subtitles
/// returns a vec of (input, output) pairs
/// allows commands to take either file paths or directory paths in agnostically
fn parse_in_out_subtitles(input: &Path, output: &Path) -> Result<Vec<(Subtitles, PathBuf)>> {
    if input.is_file() {
        Ok(vec![(read_subtitles_file(input)?, output.to_owned())])
    } else if input.is_dir() {
        Ok(list_subtitles_files(input)
            .iter()
            .map(|f| {
                let subs = read_subtitles_file(f)
                    .expect(format!("could not read subtitles file {f:#?}").as_str());
                let out_name = format!("{0}.out.srt", f.file_stem().unwrap().to_string_lossy());
                let outfile = output.join(out_name);
                (subs, outfile)
            })
            .collect())
    } else {
        Err(anyhow!(
            "input path {input:#?} was not a file or directory, are you sure it exists?"
        ))
    }
}

#[cfg(debug_assertions)]
fn debug() -> Result<()> {
    Ok(())
}

fn convert_subtitles(input: &Path, output: &Path) -> Result<()> {
    let pairs = parse_in_out_subtitles(input, output)?;

    for (subtitles, output_file) in pairs {
        subtitles.write_to_file(output_file, None)?;
    }
    Ok(())
}

fn strip_html(input: &Path, output: &Path) -> Result<()> {
    let pairs = parse_in_out_subtitles(input, output)?;
    for (mut subtitles, output_file) in pairs {
        modify::strip_html(&mut subtitles)?;
        subtitles.write_to_file(output_file, None)?;
    }
    Ok(())
}

fn fill_with_reference(
    subtitles_directory: &Path,
    videos_directory: &Path,
    create_dual_subs: bool,
    suffix: &str,
    subtitles_track: u32,
    sync_method: &SyncTool,
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
        println!("syncing sub file {sub_file:#?} with video {video_file:#?}");
        let file_stem = video_file.file_stem().unwrap();
        let video_subs = ffmpeg::extract_subtitles(video_file, subtitles_track)?;
        let unsynced_subs = SubtitleSource::File(sub_file.clone()).to_subtitles()?;
        let synced_subs = sync(&video_subs, &unsynced_subs, sync_method)?;
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

    println!("writing output files...");
    for (subtitles, output_file) in output_subs.iter() {
        subtitles.write_to_file(output_file, None)?;
    }

    println!("done!");
    Ok(())
}
