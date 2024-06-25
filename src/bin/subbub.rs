// this file contains the CLI binary for subbub

use itertools::Itertools;
use rayon::prelude::*;
use std::iter::zip;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::{fs, hash};

use anyhow::{anyhow, Error};
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use log::LevelFilter;
use srtlib::Subtitles as SrtSubtitles;
use subbub::core::data::{hash_subtitles, is_video_file, SyncTool};
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
    #[arg(short = 'l', long, default_value = "WARN", verbatim_doc_comment)]
    log_level: LevelFilter,
    /// when specified, keeps temporary files around
    #[arg(short = 'k', long, default_value = "false", verbatim_doc_comment)]
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

#[derive(Args, Debug)]
#[clap(alias = "subs")]
struct Subtitles {
    /// the subtitles used as input
    /// this may be a subtitles file, a video file, or a directory containing either subtitles files or video files
    #[arg(short = 'i', long, verbatim_doc_comment)]
    input: PathBuf,
    /// the subtitles track to use if the input is a video
    #[arg(short = 't', long, verbatim_doc_comment)]
    track: Option<u32>,
    /// the location to output the modified subtitles
    /// if the input contains multiple subtitles, this will be considered a directory, otherwise, a filename
    #[arg(short = 'o', long, verbatim_doc_comment)]
    output: PathBuf,
    #[clap(subcommand)]
    command: SubtitlesCommand,
}

#[derive(Subcommand, Debug)]
#[clap(verbatim_doc_comment)]
enum SubtitlesCommand {
    /// converts the given subtitle file(s) to srt format
    ConvertSubtitles,
    /// strips html from the given subtitle file(s)
    StripHtml,
    /// shifts the timing of the given subtitle(s) earlier or later by the given value in seconds
    ShiftTiming {
        /// the number of seconds to shift the subtitle(s)
        #[arg(short = 's', long)]
        seconds: f32,
        /// the direction to shift the subtitles
        #[arg(short = 'd', long)]
        direction: ShiftDirection,
    },
    /// syncs the timing of the given subtitles(s) to the secondary subtitle(s)
    Sync {
        /// the secondary subtitles to add to the given subtitles
        #[arg(short = 'r', long, alias = "reference")]
        reference_subtitles: PathBuf,
        /// the subtitles track, if the secondary subtitles are contained in a video
        #[arg(short = 'y', long, alias = "track2")]
        reference_track: Option<u32>,
        /// the tool to use to sync the subs
        #[arg(short = 't', long, alias = "tool", default_value = "ffsubsync")]
        sync_tool: SyncTool,
    },
    /// combines the given subtitles with another set of subtitles, creating dual subtitles (displaying both at the same time)
    /// primary subtitles will be displayed below the video
    /// secondary subtitles will be displayed above the video
    Combine {
        /// the secondary subtitles to add to the given subtitles
        #[arg(short = 's', long, alias = "secondary")]
        secondary_subtitles: PathBuf,
        /// the subtitles track, if the secondary subtitles are contained in a video
        #[arg(short = 'y', long, alias = "track2")]
        secondary_track: Option<u32>,
    },
    /// takes the subtitles from their current directory and places them alongside the videos present in the output directory
    /// also renames them to match the videos
    /// this makes the subtitles discoverable by various media library management applications
    MatchVideos {
        /// the suffix to place at the end of the subtitles file to distinguish it from other subtitle files in the same directory
        #[arg(short = 's', long)]
        suffix: Option<String>,
    },
    /// adds given subtitle(s) (-i/--input) to the given video(s) (-v/--video_path)
    AddSubtitles {
        /// the path to the video file(s) that will have subtitles added
        #[arg(short = 'v', long)]
        video_path: PathBuf,
        /// the track number that will be assigned to the newly added subtitle track
        #[arg(short = 'n', long)]
        new_track: u32,
        /// the language code that will be assigned to the newly added subtitle track
        #[arg(short = 'c', long)]
        language_code: String,
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

fn subtitles_command(_: &Commands, subcommand: &Subtitles) -> Result<()> {
    let merged_io = merge_io(&subcommand.input, subcommand.track, &subcommand.output)?;
    log::debug!("executing command {subcommand:#?}");
    match &subcommand.command {
        SubtitlesCommand::ConvertSubtitles => convert_subtitles(&merged_io)?,
        SubtitlesCommand::StripHtml => strip_html(&merged_io)?,
        SubtitlesCommand::ShiftTiming { seconds, direction } => {
            shift_seconds(&merged_io, *seconds, *direction)?
        }
        SubtitlesCommand::Sync {
            reference_subtitles,
            reference_track,
            sync_tool,
        } => sync_subs(merged_io, reference_subtitles, *reference_track, *sync_tool)?,
        SubtitlesCommand::Combine {
            secondary_subtitles,
            secondary_track,
        } => combine_subs(merged_io, secondary_subtitles, *secondary_track)?,
        SubtitlesCommand::MatchVideos { suffix } => {
            match_videos(&subcommand.input, &subcommand.output, suffix.as_deref())?
        }
        SubtitlesCommand::AddSubtitles {
            video_path,
            new_track,
            language_code,
        } => add_subtitles(
            &subcommand.input,
            subcommand.track,
            &subcommand.output,
            video_path,
            *new_track,
            language_code,
        )?,
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
        for error in errors {
            log::error!(
                "error:\n{0:#?}\nroot cause:\n{1}\nbacktrace:\n{2}",
                error,
                error.root_cause(),
                error.backtrace()
            );
        }
        Err(anyhow!("encountered errors, see logs"))
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
        for error in errors {
            log::error!(
                "error:\n{0:#?}\nroot cause:\n{1}\nbacktrace:\n{2}",
                error,
                error.root_cause(),
                error.backtrace()
            );
        }
        Err(anyhow!("encountered errors, see logs"))
    }
}

fn parse_subtitles_input(input: &Path, track: Option<u32>) -> Result<Vec<(PathBuf, SrtSubtitles)>> {
    if input.is_file() {
        log::trace!("input {input:#?} detected as single video file");
        if is_video_file(input) {
            let track = track.context(
                "when supplying a video file as input, subtitle track must be specified",
            )?;
            Ok(vec![(
                input.to_path_buf(),
                ffmpeg::extract_subtitles(input, track)?,
            )])
        } else {
            log::trace!("input {input:#?} detected as single subtitles file");
            Ok(vec![((input.to_path_buf(), read_subtitles_file(input)?))])
        }
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
            log::trace!("input {input:#?} detected as directory of video files");
            parse_videos(&videos, track.unwrap())
        } else if !subtitles.is_empty() {
            if track.is_some() {
                return Err(anyhow!(
                    "video track {0} has been specified, but command is not operating on videos",
                    track.unwrap()
                ));
            }
            log::trace!("input {input:#?} detected as directory of subtitles files");
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

#[cfg(debug_assertions)]
fn debug() -> Result<()> {
    Ok(())
}

fn convert_subtitles(merged_io: &Vec<SubtitlesIO>) -> Result<()> {
    let result: Result<()> = merged_io
        .par_iter()
        .map(|io| {
            log::debug!(
                "converting {0:#?} to {1:#?}",
                &io.input_path,
                &io.output_path
            );
            std::fs::create_dir_all(&io.output_path.parent().unwrap())?;
            io.subtitles.write_to_file(&io.output_path, None)?;
            Ok(())
        })
        .collect();
    result?;
    Ok(())
}

fn strip_html(merged_io: &Vec<SubtitlesIO>) -> Result<()> {
    let result: Result<()> = merged_io
        .par_iter()
        .map(|io| {
            let mut subs = io.subtitles.clone();
            log::debug!(
                "stripping html from {0:#?} and saving to {1:#?}",
                &io.input_path,
                &io.output_path
            );
            modify::strip_html(&mut subs)?;
            std::fs::create_dir_all(&io.output_path.parent().unwrap())?;
            subs.write_to_file(&io.output_path, None)?;
            Ok(())
        })
        .collect();
    result?;
    Ok(())
}

fn shift_seconds(
    merged_io: &Vec<SubtitlesIO>,
    mut seconds: f32,
    direction: ShiftDirection,
) -> Result<()> {
    match direction {
        ShiftDirection::EARLIER => seconds = -1.0 * seconds,
        _ => (),
    }
    let result: Result<()> = merged_io
        .par_iter()
        .map(|io| {
            let subtitles = &io.subtitles;
            log::debug!(
                "shifting timing of {0:#?} and saving to {1:#?}",
                &io.input_path,
                &io.output_path
            );
            let shifted = modify::shift_seconds(subtitles, seconds)?;
            std::fs::create_dir_all(&io.output_path.parent().unwrap())?;
            shifted.write_to_file(&io.output_path, None)?;
            Ok(())
        })
        .collect();
    result?;
    Ok(())
}

fn combine_subs(
    mut merged_io: Vec<SubtitlesIO>,
    secondary_subtitles: &Path,
    secondary_track: Option<u32>,
) -> Result<()> {
    let mut secondary_input = parse_subtitles_input(secondary_subtitles, secondary_track)?;
    if secondary_input.len() != merged_io.len() {
        return Err(anyhow!("primary and secondary subtitle inputs have different lengths, cannot match them to combine:\n    primary: {0}\n    secondary: {1}", merged_io.len(), secondary_input.len()));
    }

    // sort to make sure we match the correct pairs
    merged_io.sort_by_key(|io| io.input_path.clone());
    secondary_input.sort_by_key(|i| i.0.clone());

    let zipped = zip(merged_io, secondary_input);
    let result: Result<()> = zipped
        .par_bridge()
        .map(|(io, (secondary_input, secondary_subtitles))| {
            log::debug!(
                "combining {0:#?} with {1:#?} and saving to {2:#?}",
                &io.input_path,
                &secondary_input,
                &io.output_path
            );
            std::fs::create_dir_all(&io.output_path.parent().unwrap())?;
            let primary_subtitles = &io.subtitles;
            let output_path = &io.output_path;
            let merged_subs = merge(&primary_subtitles, &secondary_subtitles)?;
            merged_subs.write_to_file(output_path, None)?;
            Ok(())
        })
        .collect();
    result?;

    Ok(())
}

fn match_videos(input: &Path, output: &Path, suffix: Option<&str>) -> Result<()> {
    let parent_dir = input.file_stem().unwrap().to_string_lossy();
    let default_extension = format!(".{0}", parent_dir);
    let suffix_str = suffix.unwrap_or_else(|| &default_extension);
    let mut inputs = list_subtitles_files(input);
    let mut videos = list_video_files(output);

    if inputs.len() != videos.len() {
        return Err(anyhow!("number of subtitles and number of videos are not the same:\n    videos: {0}\n    subtitles: {1}", videos.len(), inputs.len()));
    }

    inputs.sort();
    videos.sort();

    let result: Result<()> = zip(inputs, videos)
        .par_bridge()
        .map(|(subtitle, video)| {
            let video_name = video.file_stem().unwrap();
            let output_filename = PathBuf::from(format!(
                "{0}{1}.srt",
                output.join(video_name).to_string_lossy(),
                suffix_str
            ));
            std::fs::copy(subtitle, output_filename)?;
            Ok(())
        })
        .collect();

    result?;

    Ok(())
}

fn sync_subs(
    mut merged_io: Vec<SubtitlesIO>,
    reference_subtitles: &Path,
    reference_track: Option<u32>,
    sync_tool: SyncTool,
) -> Result<()> {
    let mut secondary_input = parse_subtitles_input(reference_subtitles, reference_track)?;
    if secondary_input.len() != merged_io.len() {
        return Err(anyhow!("primary and secondary subtitle inputs have different lengths, cannot match them to combine:\n    primary: {0}\n    secondary: {1}", merged_io.len(), secondary_input.len()));
    }

    // sort to make sure we match the correct pairs
    merged_io.sort_by_key(|io| io.input_path.clone());
    secondary_input.sort_by_key(|i| i.0.clone());

    let zipped: Vec<_> = zip(merged_io, secondary_input).collect();
    let result: Result<()> = zipped
        .par_iter()
        .map(|(io, (reference_input, reference_subtitles))| {
            log::debug!(
                "syncing {0:#?} with {1:#?} and saving to {2:#?}",
                &io.input_path,
                &reference_input,
                &io.output_path
            );
            std::fs::create_dir_all(&io.output_path.parent().unwrap())?;
            let primary_subtitles = &io.subtitles;
            let output_path = &io.output_path;
            let synced_subs = sync(&reference_subtitles, &primary_subtitles, &sync_tool)?;
            synced_subs.write_to_file(output_path, None)?;
            Ok(())
        })
        .collect();
    result?;

    Ok(())
}

fn add_subtitles(
    input: &Path,
    input_track: Option<u32>,
    output: &Path,
    videos_path: &Path,
    new_track: u32,
    language_code: &str,
) -> Result<()> {
    let mut subtitles = parse_subtitles_input(input, input_track)?;

    let mut videos = if videos_path.is_dir() {
        list_video_files(videos_path)
    } else {
        vec![videos_path.to_path_buf()]
    };

    if videos.len() != subtitles.len() {
        return Err(anyhow!("subtitles and video inputs have different lengths, cannot match them to combine:\n    subtitle: {0}\n    video: {1}", subtitles.len(), videos.len()));
    }

    videos.sort();
    subtitles.sort_by_key(|(path, _)| path.clone());

    let units = zip(subtitles, videos).collect_vec();
    for ((input_path, subtitles), video_path) in units {
        // get subtitles path on disk
        let subtitles_path = if is_video_file(&input_path) {
            let tmp_filename = format!("add_{0}.srt", hash_subtitles(&subtitles));
            let tmp_filepath = TMP_DIRECTORY.get().unwrap().join(tmp_filename);
            // if input path is a video file, we'll need to save the extracted subs and point to the extracted path
            subtitles.write_to_file(&tmp_filepath, None)?;
            tmp_filepath
        } else {
            // if input path is not a video file, we can assume it's a subtitles file and point to the path
            input_path
        };

        let output_path = if subtitles.len() == 1 {
            // if there's only one input, the output should be a single file
            fs::create_dir_all(output.parent().context("output path has no parent")?)?;
            output.to_path_buf()
        } else {
            // if there are multiple inputs, we'll use the given output as a directory, and name the output videos the same as their input counterpart
            fs::create_dir_all(output)?;
            let filename = video_path
                .file_name()
                .context("video file has no file name")?;
            output.join(filename)
        };
        ffmpeg::add_subtitles_track(
            &video_path,
            &subtitles_path,
            new_track,
            language_code,
            &output_path,
        )?;
    }

    Ok(())
}
