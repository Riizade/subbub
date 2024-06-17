// this file contains the CLI binary for subbub

use std::path::{Path, PathBuf};
use std::process::exit;

use anyhow::Result;
use anyhow::{anyhow, Error};
use clap::{Args, Parser, Subcommand};
use itertools::{Either, Itertools};
use log::LevelFilter;
use srtlib::Subtitles as SrtSubtitles;
use subbub::core::data::SyncTool;
use subbub::core::data::{list_subtitles_files, list_video_files, TMP_DIRECTORY};
use subbub::core::data::{ShiftDirection, SubtitleSource};
use subbub::core::ffmpeg;
use subbub::core::ffmpeg::read_subtitles_file;
use subbub::core::log::initialize_logging;
use subbub::core::merge::merge;
use subbub::core::modify;
use subbub::core::sync::sync;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    /// overrides the log level
    #[arg(short = 'l', long, default_value = "WARN")]
    log_level: LevelFilter,
    /// when specified, keeps temporary files around
    #[arg(short = 'k', long, default_value = "false")]
    keep_tmp_files: bool,
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// commands to modify subtitles
    Subtitles(Subtitles),
    /// command for testing
    #[cfg(debug_assertions)]
    Debug,
}

#[derive(Args)]
#[clap(alias = "subs")]
struct Subtitles {
    /// the subtitles used as input
    /// this may be a subtitles file, a video file, or a directory containing either subtitles files or video files
    #[arg(short = 'i', long)]
    input: PathBuf,
    /// the subtitles track to use if the input is a video
    #[arg(short = 't', long)]
    track: Option<u32>,
    /// the location to output the modified subtitles
    /// if the input contains multiple subtitles, this will be considered a directory, otherwise, a filename
    #[arg(short = 'o', long)]
    output: PathBuf,
    #[clap(subcommand)]
    command: SubtitlesCommand,
}

#[derive(Subcommand)]
enum SubtitlesCommand {
    /// converts the given subtitle file(s) to srt format
    ConvertSubtitles,
    /// strips html from the given subtitle file(s)
    StripHtml,
    /// shifts the timing of the given subtitle(s) earlier or later by the given value in seconds
    ShiftTiming {
        /// the number of seconds to shift the subtitle(s) earlier or later
        #[arg(short = 's', long)]
        seconds: f32,
        /// can use + for later, - for earlier
        #[arg(short = 'd', long)]
        direction: ShiftDirection,
    },
    /// combines the given subtitles with another set of subtitles, creating dual subtitles (displaying both at the same time)
    /// primary subtitles will be displayed below the video
    /// secondary subtitles will be displayed above the video
    Combine {
        /// the secondary subtitles to add to the given subtitles
        #[arg(short = 's', long, alias = "secondary")]
        secondary_subtitles: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    initialize_logging(cli.log_level);

    let result = match &cli.command {
        Commands::Subtitles(subcommand) => subtitles_command(&cli.command, subcommand),
        #[cfg(debug_assertions)]
        Commands::Debug => debug(),
    };

    // clean up
    if !cli.keep_tmp_files {
        let tmp_dir = TMP_DIRECTORY.get().unwrap();
        if tmp_dir.exists() {
            std::fs::remove_dir_all(TMP_DIRECTORY.get().unwrap())
                .expect("could not remove tmp directory");
        }
    }

    match result {
        Ok(_) => println!("done!"),
        Err(e) => {
            println!("command execution failed:\nerror: {0}\nsource: {1:#?}\nroot cause: {2}\nbacktrace: {3}", e, e.source(), e.root_cause(), e.backtrace());
            exit(1);
        }
    }
}

struct SubtitlesIO {
    input_path: PathBuf,
    subtitles: SrtSubtitles,
    output_path: PathBuf,
}

impl SubtitlesIO {}

fn subtitles_command(command: &Commands, subcommand: &Subtitles) -> Result<()> {
    let merged_io = merge_io(&subcommand.input, subcommand.track, &subcommand.output)?;
    match &subcommand.command {
        SubtitlesCommand::ConvertSubtitles => convert_subtitles(&merged_io)?,
        SubtitlesCommand::StripHtml => strip_html(&merged_io)?,
        SubtitlesCommand::ShiftTiming { seconds, direction } => todo!(),
        SubtitlesCommand::Combine {
            secondary_subtitles,
        } => todo!(),
    }
    Ok(())
}

fn merge_io(input: &Path, track: Option<u32>, output: &Path) -> Result<Vec<SubtitlesIO>> {
    let input_subs = parse_subtitles_input(input, track)?;
    if input_subs.len() == 1 {
        // if there is exactly one entry, the output path is used as a filename
        let (path, subs) = input_subs.first().unwrap();
        Ok(vec![SubtitlesIO {
            input_path: path.to_path_buf(),
            subtitles: subs.clone(),
            output_path: output.to_path_buf(),
        }])
    } else {
        Ok(input_subs
            .iter()
            .map(|(input_path, subtitles)| {
                let output_path = output.join(input_path.file_name().unwrap());
                SubtitlesIO {
                    input_path: input_path.to_path_buf(),
                    subtitles: subtitles.clone(),
                    output_path,
                }
            })
            .collect())
    }
}

fn parse_videos(videos: &Vec<PathBuf>, track: u32) -> Result<Vec<(PathBuf, SrtSubtitles)>> {
    let mut subs: Vec<(PathBuf, SrtSubtitles)> = vec![];
    let mut errors: Vec<Error> = vec![];
    videos.iter().for_each(|v| {
        let result = ffmpeg::extract_subtitles(v, track);
        match result {
            Ok(s) => subs.push((v.to_path_buf(), s)),
            Err(e) => errors.push(e),
        }
    });
    if errors.is_empty() {
        Ok(subs)
    } else {
        Err(anyhow!("encountered errors"))
    }
}

fn parse_subtitles(subtitles: &Vec<PathBuf>) -> Result<Vec<(PathBuf, SrtSubtitles)>> {
    let mut subs: Vec<(PathBuf, SrtSubtitles)> = vec![];
    let mut errors: Vec<Error> = vec![];
    subtitles.iter().for_each(|sub| {
        let result = ffmpeg::read_subtitles_file(sub);
        match result {
            Ok(s) => subs.push((sub.to_path_buf(), s)),
            Err(e) => errors.push(e),
        }
    });
    if errors.is_empty() {
        Ok(subs)
    } else {
        Err(anyhow!("encountered errors"))
    }
}

fn parse_subtitles_input(input: &Path, track: Option<u32>) -> Result<Vec<(PathBuf, SrtSubtitles)>> {
    if input.is_file() {
        Ok(vec![((input.to_path_buf(), read_subtitles_file(input)?))])
    } else if input.is_dir() {
        let videos = list_video_files(input);
        let subtitles = list_subtitles_files(input);
        if videos.is_empty() && subtitles.is_empty() {
            Err(anyhow!(
                "input directory does not contain any video or subtitles files"
            ))
        } else if !videos.is_empty() && !subtitles.is_empty() {
            Err(anyhow!("input directory contains both videos and subtitles, should contain only one or the other"))
        } else if !videos.is_empty() {
            if track.is_none() {
                return Err(anyhow!(
                    "when using subtitles from a video file, the track must be specified"
                ));
            }
            parse_videos(&videos, track.unwrap())
        } else if !subtitles.is_empty() {
            if track.is_some() {
                return Err(anyhow!(
                    "video track {0} has been specified, but command is not operating on videos",
                    track.unwrap()
                ));
            }
            parse_subtitles(&subtitles)
        } else {
            unreachable!();
        }
    } else {
        Err(anyhow!(
            "input path {input:#?} was not a file or directory, are you sure it exists?"
        ))
    }
}

fn subtitles_output() {}

#[cfg(debug_assertions)]
fn debug() -> Result<()> {
    Ok(())
}

fn convert_subtitles(merged_io: &Vec<SubtitlesIO>) -> Result<()> {
    for io in merged_io {
        io.subtitles.write_to_file(io.output_path, None)?;
    }
    Ok(())
}

fn strip_html(merged_io: &Vec<SubtitlesIO>) -> Result<()> {
    for io in merged_io {
        let mut subs = io.subtitles.clone();
        modify::strip_html(&mut subs)?;
        subs.write_to_file(io.output_path, None)?;
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

    let mut output_subs: Vec<(SrtSubtitles, PathBuf)> = vec![];

    // TODO: remove debug prints
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

    Ok(())
}
