// this file contains the CLI binary for subbub

use itertools::Itertools;
use rayon::prelude::*;
use rayon::str::Bytes;
use std::f32::consts::E;
use std::io::Write;
use std::iter::zip;
use std::path::{Path, PathBuf};
use std::process::{exit, Output};
use std::{fs, hash};

use anyhow::{anyhow, Context, Error, Result};
use clap::{ArgGroup, Args, Parser, Subcommand};
use log::LevelFilter;
use srtlib::Subtitles as SrtSubtitles;
use subbub::core::data::{hash_subtitles, is_video_file, SyncTool, VideoSource};
use subbub::core::data::{list_subtitles_files, list_video_files, TMP_DIRECTORY};
use subbub::core::data::{ShiftDirection, SubtitleSource};
use subbub::core::ffmpeg::read_subtitles_file;
use subbub::core::log::initialize_logging;
use subbub::core::merge::merge;
use subbub::core::modify::{self, strip_html};
use subbub::core::sync::sync;
use subbub::core::{ffmpeg, mkvmerge};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    /// overrides the log level
    #[arg(short = 'l', long, default_value = "INFO", verbatim_doc_comment)]
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
    CompoundOperations(CompoundOperations),
    /// command for testing
    #[cfg(debug_assertions)]
    Debug,
}

#[derive(Args, Debug)]
#[clap(group(ArgGroup::new("subtitle").required(true).multiple(false)))]
struct SubtitleArgs {
    /// the input file or directory containing subtitles
    /// for subtitle tracks contained video files, use the format {filename}:{track_number}
    #[arg(short = 's', long, verbatim_doc_comment)]
    subtitles_path: String,
}

impl SubtitleArgs {
    /// parses the input subtitles path and returns a `SubtitleSource`
    fn parse(&self) -> Result<SubtitleSource> {
        SubtitleSource::try_from(self.subtitles_path.as_str())
    }
}

#[derive(Args, Debug)]
#[clap(group(ArgGroup::new("video").required(true).multiple(false)))]
struct VideoArgs {
    /// the input file or directory containing video file(s)
    #[arg(short = 'v', long, verbatim_doc_comment)]
    video_path: String,
}

impl TryInto<VideoSource> for VideoArgs {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<VideoSource> {
        VideoSource::try_from(self.video_path.as_str())
    }
}

impl VideoArgs {
    /// parses the input video path and returns a `PathBuf`
    fn parse(&self) -> Result<VideoSource> {
        VideoSource::try_from(self.video_path.as_str())
    }
}

#[derive(Args, Debug)]
#[clap(group(ArgGroup::new("output").required(true).multiple(false)))]
struct OutputArgs {
    /// the output file or directory where the modified entities will be saved
    #[arg(short = 'o', long, verbatim_doc_comment)]
    output: PathBuf,
}

#[derive(Args, Debug)]
#[clap(group(ArgGroup::new("language_code").required(true).multiple(false)))]
struct LanguageCodeArgs {
    /// the language code to assign to the subtitle track(s)
    #[arg(short = 'l', long, verbatim_doc_comment)]
    language_code: String,
}

#[derive(Args, Debug)]
#[clap(alias = "subs")]
struct Subtitles {
    #[clap(subcommand)]
    command: SubtitlesCommand,
}

#[derive(Subcommand, Debug)]
#[clap(verbatim_doc_comment)]
enum SubtitlesCommand {
    /// converts the given subtitle file(s) to srt format
    #[clap(verbatim_doc_comment)]
    ConvertSubtitles {
        #[command(flatten)]
        input: SubtitleArgs,
        #[command(flatten)]
        output: OutputArgs,
    },
    /// strips html from the given subtitle file(s)
    #[clap(verbatim_doc_comment)]
    StripHtml {
        #[command(flatten)]
        input: SubtitleArgs,
        #[command(flatten)]
        output: OutputArgs,
    },
    /// shifts the timing of the given subtitle(s) earlier or later by the given value in seconds
    #[clap(verbatim_doc_comment)]
    ShiftTiming {
        #[command(flatten)]
        input: SubtitleArgs,
        #[command(flatten)]
        output: OutputArgs,
        /// the number of seconds to shift the subtitle(s)
        #[arg(short = 'n', long)]
        seconds: f32,
        /// the direction to shift the subtitles
        #[arg(short = 'd', long)]
        direction: ShiftDirection,
    },
    /// syncs the timing of the given subtitles(s) to the secondary subtitle(s)
    #[clap(verbatim_doc_comment)]
    Sync {
        #[command(flatten)]
        input: SubtitleArgs,
        #[command(flatten)]
        output: OutputArgs,
        /// the subtitles to use as a timing reference for the given subtitles
        /// uses the same specification format as the input subtitles
        #[arg(short = 'r', long, visible_alias = "reference")]
        reference_subtitles: String,
        /// the tool to use to sync the subs
        /// currently the only tool available is `ffsubsync` (default)
        #[arg(short = 't', long, visible_alias = "tool", default_value = "ffsubsync")]
        sync_tool: SyncTool,
    },
    /// combines the given subtitles with another set of subtitles, creating dual subtitles (displaying both at the same time)
    /// primary subtitles will be displayed below the video
    /// secondary subtitles will be displayed above the video
    #[clap(verbatim_doc_comment)]
    Combine {
        #[command(flatten)]
        input: SubtitleArgs,
        #[command(flatten)]
        output: OutputArgs,
        /// the secondary subtitles to add to the given subtitles
        /// uses the same specification format as the input subtitles
        #[arg(short = 'e', long, visible_alias = "secondary")]
        secondary_subtitles: String,
    },
    /// takes the subtitles from their current directory and places them alongside the videos present in the output directory
    /// also renames them to match the videos
    /// this makes the subtitles discoverable by various media library management applications
    #[clap(verbatim_doc_comment)]
    MatchVideos {
        #[command(flatten)]
        input: SubtitleArgs,
        #[command(flatten)]
        output: OutputArgs,
        #[command(flatten)]
        video_path: VideoArgs,
        /// the suffix to place at the end of the subtitles file to distinguish it from other subtitle files in the same directory
        #[arg(short = 's', long)]
        suffix: Option<String>,
    },
    /// adds given subtitle(s) (-s/--subtitles) to the given video(s) (-v/--video_path)
    #[clap(verbatim_doc_comment)]
    AddSubtitles {
        #[command(flatten)]
        input: SubtitleArgs,
        #[command(flatten)]
        output: OutputArgs,
        #[command(flatten)]
        video_path: VideoArgs,
        /// the language code that will be assigned to the newly added subtitle track
        #[arg(short = 'c', long)]
        language_code: String,
    },
}

#[derive(Args, Debug)]
#[clap(visible_aliases = ["ops", "compound"])]
struct CompoundOperations {
    #[clap(subcommand)]
    command: CompoundOperationsCommand,
}

#[derive(Subcommand, Debug)]
#[clap(verbatim_doc_comment)]
/// subcommands for common sequences of operations
enum CompoundOperationsCommand {
    /// merges a directory of videos with a directory of subtitles
    /// adds the subtitles to the video both as a single sub track, and as a dual sub track
    /// this command performs auxiliary operations such as format conversion and subtitle syncing
    #[clap(verbatim_doc_comment)]
    AddDualSubs {
        /// the directory containing the video files
        #[clap(verbatim_doc_comment)]
        #[arg(short = 'v', long)]
        videos_path: PathBuf,
        /// the subtitles track in the video to use as a timing reference
        #[clap(verbatim_doc_comment)]
        #[arg(short = 't', long, visible_alias = "track")]
        subtitles_track: u32,
        /// the directory containing the subtitles files
        #[clap(verbatim_doc_comment)]
        #[arg(short = 's', long)]
        subtitles_path: PathBuf,
        /// the directory to output the newly created videos to
        /// WARNING: if you use the same directory as videos_path, the videos may be overwritten
        #[clap(verbatim_doc_comment)]
        #[arg(short = 'o', long)]
        output_path: PathBuf,
        /// the language code of the newly added subtitles file
        #[clap(verbatim_doc_comment)]
        #[arg(short = 'c', long, visible_alias = "lang")]
        language_code: String,
    },
}

fn main() {
    let cli = Cli::parse();

    initialize_logging(cli.log_level);

    let result = match &cli.command {
        Commands::Subtitles(subtitles) => subtitles_command(&cli.command, subtitles),
        Commands::CompoundOperations(operations) => operations_command(&cli.command, operations),
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

fn subtitles_command(_: &Commands, subcommand: &Subtitles) -> Result<()> {
    log::debug!("executing command {subcommand:#?}");
    match &subcommand.command {
        SubtitlesCommand::ConvertSubtitles { input, output } => convert_subtitles(&input, &output)?,
        SubtitlesCommand::StripHtml { input, output } => strip_html_from_dir(&input, &output)?,
        SubtitlesCommand::ShiftTiming {
            input,
            output,
            seconds,
            direction,
        } => shift_seconds(&input, &output, *seconds, *direction)?,
        SubtitlesCommand::Sync {
            input,
            output,
            reference_subtitles,
            sync_tool,
        } => sync_subs(input, output, reference_subtitles, *sync_tool)?,
        SubtitlesCommand::Combine {
            input,
            output,
            secondary_subtitles,
        } => combine_subs(input, output, secondary_subtitles)?,
        SubtitlesCommand::MatchVideos {
            input,
            output,
            video_path,
            suffix,
        } => match_videos(input, output, video_path, suffix.as_deref())?,
        SubtitlesCommand::AddSubtitles {
            input,
            output,
            video_path,
            language_code,
        } => add_subtitles(input, output, video_path, language_code)?,
    }
    Ok(())
}

#[cfg(debug_assertions)]
fn debug() -> Result<()> {
    Ok(())
}

fn convert_subtitles(input: &SubtitleArgs, output: &OutputArgs) -> Result<()> {
    let input_subs = input.parse()?.to_subtitles()?;
    let output_path = output.output.as_path();
    std::fs::create_dir_all(output_path)?;
    let bytes: Vec<(&Path, Vec<u8>)> = input_subs
        .par_iter()
        .map(|subtitles| {
            log::debug!(
                "converting subtitles from {0:#?} to {1:#?}",
                subtitles.path,
                output_path
            );
            (
                subtitles.path.as_path(),
                subtitles.subtitles_string().into_bytes(),
            )
        })
        .collect();
    write_to_output(output_path, &bytes)?;
    Ok(())
}

fn strip_html_from_dir(input: &SubtitleArgs, output: &OutputArgs) -> Result<()> {
    let mut input_subs = input.parse()?.to_subtitles()?;
    let output_path = output.output.as_path();
    std::fs::create_dir_all(output_path)?;
    let results: Result<Vec<(&Path, Vec<u8>)>> = input_subs
        .par_iter_mut()
        .map(|subtitles| {
            log::debug!(
                "stripping html from subtitles at {0:#?} and saving to {1:#?}",
                subtitles.path,
                output_path
            );
            modify::strip_html(&mut subtitles.subtitles)?;
            Ok((
                subtitles.path.as_path(),
                subtitles.subtitles_string().into_bytes(),
            ))
        })
        .collect();
    write_to_output(output_path, &results?)?;
    Ok(())
}

fn shift_seconds(
    input: &SubtitleArgs,
    output: &OutputArgs,
    mut seconds: f32,
    direction: ShiftDirection,
) -> Result<()> {
    match direction {
        ShiftDirection::EARLIER => seconds = -1.0 * seconds,
        _ => (),
    }

    let mut input_subs = input.parse()?.to_subtitles()?;
    let output_path = output.output.as_path();
    std::fs::create_dir_all(output_path)?;
    let results: Result<Vec<(&Path, Vec<u8>)>> = input_subs
        .par_iter_mut()
        .map(|subtitles| {
            log::debug!(
                "shifting timing of {0:#?} and saving to {1:#?}",
                subtitles.path,
                output_path
            );
            Ok((
                subtitles.path.as_path(),
                modify::shift_seconds(&subtitles.subtitles, seconds)?
                    .to_string()
                    .into_bytes(),
            ))
        })
        .collect();
    write_to_output(output_path, &results?)?;
    Ok(())
}

fn combine_subs(
    input: &SubtitleArgs,
    output: &OutputArgs,
    secondary_subtitles_string: &str,
) -> Result<()> {
    let mut primary_subtitles = input.parse()?.to_subtitles()?;
    let mut secondary_subtitles =
        SubtitleSource::try_from(secondary_subtitles_string)?.to_subtitles()?;

    if primary_subtitles.len() != secondary_subtitles.len() {
        return Err(anyhow!(
            "primary and secondary subtitle inputs have different lengths, cannot match them to combine:\n    primary: {0}\n    secondary: {1}",
            primary_subtitles.len(),
            secondary_subtitles.len()
        ));
    }

    // sort to make sure we match the correct pairs
    primary_subtitles.sort();
    secondary_subtitles.sort();

    let zipped = zip(primary_subtitles, secondary_subtitles).collect::<Vec<_>>();
    let result: Result<Vec<(&Path, Vec<u8>)>> = zipped
        .par_iter()
        .map(|(primary, secondary)| {
            log::debug!(
                "combining {0:#?} with {1:#?} and saving to {2:#?}",
                &primary.path,
                &secondary.path,
                &output.output
            );
            let merged_subs = merge(&primary.subtitles, &secondary.subtitles)?;
            let bytes = merged_subs.to_string().into_bytes();
            Ok((primary.path.as_path(), bytes))
        })
        .collect();

    write_to_output(&output.output, &result?)?;

    Ok(())
}

fn match_videos(
    input: &SubtitleArgs,
    output: &OutputArgs,
    video_path: &VideoArgs,
    suffix: Option<&str>,
) -> Result<()> {
    let mut input_subs = input.parse()?.to_subtitles()?;

    let parent_dir = input_subs
        .first()
        .expect("input subtitles does not contain any subtitles files")
        .path
        .file_stem()
        .expect("input subtitles has no file stem")
        .to_string_lossy();
    let default_extension = format!(".{0}", parent_dir);
    let suffix_str = suffix.unwrap_or_else(|| &default_extension);
    let mut videos = video_path.parse()?.to_videos()?;

    if input_subs.len() != videos.len() {
        return Err(anyhow!("number of subtitles and number of videos are not the same:\n    videos: {0}\n    subtitles: {1}", videos.len(), input_subs.len()));
    }

    input_subs.sort();
    videos.sort();

    let result: Result<()> = zip(input_subs, videos)
        .par_bridge()
        .map(|(subtitle, video)| {
            let video_name = video.file_stem().unwrap();
            let output_filename = PathBuf::from(format!(
                "{0}{1}.srt",
                output.output.join(video_name).to_string_lossy(),
                suffix_str
            ));
            std::fs::copy(subtitle.path, output_filename)?;
            Ok(())
        })
        .collect();

    result?;

    Ok(())
}

fn sync_subs(
    input: &SubtitleArgs,
    output: &OutputArgs,
    reference_subtitles: &str,
    sync_tool: SyncTool,
) -> Result<()> {
    let mut input_subs = input.parse()?.to_subtitles()?;
    let mut reference_subs = SubtitleSource::try_from(reference_subtitles)?.to_subtitles()?;
    if reference_subs.len() != input_subs.len() {
        return Err(anyhow!("primary and secondary subtitle inputs have different lengths, cannot match them to combine:\n    primary: {0}\n    reference: {1}", input_subs.len(), reference_subs.len()));
    }

    // sort to make sure we match the correct pairs
    input_subs.sort();
    reference_subs.sort();

    let zipped: Vec<_> = zip(input_subs, reference_subs).collect();
    let result: Result<Vec<(&Path, Vec<u8>)>> = zipped
        .par_iter()
        .map(|(primary, reference)| {
            log::debug!(
                "syncing {0:#?} with {1:#?} and saving to {2:#?}",
                &primary.path,
                &reference.path,
                &output.output
            );
            std::fs::create_dir_all(&output.output.parent().unwrap())?;
            let synced_subs = sync(&reference.subtitles, &primary.subtitles, &sync_tool)?;

            Ok((primary.path.as_path(), synced_subs.to_string().into_bytes()))
        })
        .collect();

    write_to_output(&output.output, &result?)?;

    Ok(())
}

fn add_subtitles(
    input: &SubtitleArgs,
    output: &OutputArgs,
    video_path: &VideoArgs,
    language_code: &str,
) -> Result<()> {
    let mut subtitles = input.parse()?.to_subtitles()?;
    let mut videos = video_path.parse()?.to_videos()?;
    if subtitles.len() != videos.len() {
        return Err(anyhow!(
            "subtitles and video inputs have different lengths, cannot match them to combine:\n    subtitles: {0}\n    videos: {1}",
            subtitles.len(),
            videos.len()
        ));
    }

    videos.sort();
    subtitles.sort();

    let units = zip(&subtitles, videos).collect_vec();
    for (subs, video_path) in units {
        // get subtitles path on disk
        let subtitles_path = if is_video_file(&video_path) {
            let tmp_filename = format!("add_{0}.srt", hash_subtitles(&subs.subtitles));
            let tmp_filepath = TMP_DIRECTORY.get().unwrap().join(tmp_filename);
            // if input path is a video file, we'll need to save the extracted subs and point to the extracted path
            subs.subtitles.write_to_file(&tmp_filepath, None)?;
            tmp_filepath
        } else {
            // if input path is not a video file, we can assume it's a subtitles file and point to the path
            subs.path.clone()
        };

        let output_path = if subtitles.len() == 1 {
            // if there's only one input, the output should be a single file
            fs::create_dir_all(
                output
                    .output
                    .parent()
                    .context("output path has no parent")?,
            )?;
            output.output.to_path_buf()
        } else {
            // if there are multiple inputs, we'll use the given output as a directory, and name the output videos the same as their input counterpart
            fs::create_dir_all(output.output.clone())?;
            let filename = video_path
                .file_name()
                .context("video file has no file name")?;
            output.output.join(filename)
        };
        mkvmerge::add_subtitles_track(
            &video_path,
            &subtitles_path,
            Some(language_code),
            language_code,
            &output_path,
        )?;
    }

    Ok(())
}

fn operations_command(_: &Commands, operations: &CompoundOperations) -> Result<()> {
    match &operations.command {
        CompoundOperationsCommand::AddDualSubs {
            videos_path,
            subtitles_track,
            subtitles_path,
            output_path,
            language_code,
        } => dual_subs_command(
            &videos_path,
            &subtitles_path,
            *subtitles_track,
            &language_code,
            &output_path,
        ),
    }?;

    Ok(())
}

fn dual_subs_command(
    videos_path: &Path,
    subtitles_path: &Path,
    track: u32,
    language_code: &str,
    output: &Path,
) -> Result<()> {
    if videos_path.canonicalize()? == output.canonicalize()? {
        return Err(anyhow!("videos path and output path are the same, this could cause overwriting of the original video files\nplease choose a different output path"));
    }

    let mut video_files = list_video_files(videos_path);
    let mut subtitles_files = list_subtitles_files(subtitles_path);

    if video_files.len() != subtitles_files.len() {
        return Err(anyhow!(
            "video and subtitle counts do not match; videos: {0}, subtitles: {1}",
            video_files.len(),
            subtitles_files.len()
        ));
    }

    video_files.sort();
    subtitles_files.sort();

    let zipped = zip(video_files, subtitles_files).collect::<Vec<_>>();
    let errors = zipped
        .par_iter()
        .enumerate()
        .map(|tuple: (usize, &(PathBuf, PathBuf))| {
            dual_subs_command_single(tuple, track, language_code, output)
        })
        .filter(|r| r.is_err())
        .map(|r| r.err().unwrap())
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        let mut error_vec: Vec<u8> = vec![];
        for error in errors {
            writeln!(error_vec, "{error}")?;
        }
        return Err(anyhow!(
            "one or more operations not successful:\n{0}",
            String::from_utf8(error_vec)?
        ));
    }

    log::info!("done! finished processing all videos");

    Ok(())
}

fn dual_subs_command_single(
    tuple: (usize, &(PathBuf, PathBuf)),
    track: u32,
    language_code: &str,
    output: &Path,
) -> Result<()> {
    let (index, (video_file, subtitles_file)) = tuple;
    log::info!("started processing video #{index}");
    let video_filename = video_file.file_stem().unwrap().to_string_lossy();

    // convert video to mkv
    log::info!("#{index}: converting video to mkv...");
    let mkv_filepath = ffmpeg::convert_to_mkv(video_file)?;
    // extract provided track number
    log::info!("#{index}: extracting reference subs...");
    let mut subs_from_video = ffmpeg::extract_subtitles(video_file, track)?;
    // convert provided subs to srt and sync
    // surround in a scope block so that we don't accidentally use the raw subs_from_file in later steps
    let mut synced_subs_from_file = {
        log::info!("#{index}: converting subs to srt...");
        let subs_from_file = ffmpeg::read_subtitles_file(&subtitles_file)?;
        // sync subs
        log::info!("#{index}: syncing subs...");
        sync(&subs_from_video, &subs_from_file, &SyncTool::FFSUBSYNC)?
    };
    log::info!("#{index}: stripping HTML from subs...");
    strip_html(&mut subs_from_video)?;
    strip_html(&mut synced_subs_from_file)?;
    // combine provided subs with extracted track
    log::info!("#{index}: merging subs...");
    let merged_subs = merge(&subs_from_video, &synced_subs_from_file)?;

    // add sub tracks to mkv file

    // determine temporary filepaths for subs and videos
    let intermediate_video = TMP_DIRECTORY
        .get()
        .unwrap()
        .join(format!("{0}-intermediate.mkv", video_filename));
    let single_sub_filepath = TMP_DIRECTORY
        .get()
        .unwrap()
        .join(format!("{0}-single.srt", video_filename));
    synced_subs_from_file.write_to_file(&single_sub_filepath, None)?;
    let dual_sub_filepath = TMP_DIRECTORY
        .get()
        .unwrap()
        .join(format!("{0}-dual.srt", video_filename));
    merged_subs.write_to_file(&dual_sub_filepath, None)?;

    // add single sub track
    log::info!("#{index}: adding single subs track...");
    mkvmerge::add_subtitles_track(
        &mkv_filepath,
        &single_sub_filepath,
        Some(language_code),
        language_code,
        &intermediate_video,
    )?;
    // add dual sub track
    log::info!("#{index}: adding dual subs track...");
    let final_video = output.join(format!("{0}.mkv", video_filename));
    std::fs::create_dir_all(output)?;
    mkvmerge::add_subtitles_track(
        &intermediate_video,
        &dual_sub_filepath,
        None,
        format!("dual-{language_code}").as_str(),
        &final_video,
    )?;
    log::info!("finished processing video #{index}");
    Ok(())
}

/// writes the given collection of (path, byte strings) to files in the output directory using the original file names.
/// If there is only one file, it writes it directly to the output path.
fn write_to_output(output: &Path, files: &Vec<(&Path, Vec<u8>)>) -> Result<()> {
    if files.is_empty() {
        return Err(anyhow!("no files to write to output"));
    } else if files.len() == 1 {
        // if there's only one file, write it directly to the output path
        let mut file = fs::File::create(output).context("could not create output file")?;
        file.write_all(&files[0].1)
            .context("could not write to output file")?;
        return Ok(());
    } else {
        // if there are multiple files, write them to the output directory
        for (original_file, bytes) in files {
            let destination_file =
                output.join(original_file.file_name().context("file has no name")?);
            let mut file = fs::File::create(&destination_file)
                .context(format!("could not create file {destination_file:#?}"))?;
            file.write_all(bytes)
                .context(format!("could not write to file {destination_file:#?}"))?;
        }
    }
    Ok(())
}
