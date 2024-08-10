# subbub

A CLI frontend for using mkvextract, ffsubsync, dualsub, etc to manage and merge subtitles

# Requirements

Must have the following installed and available in PATH

- https://ffmpeg.org/
  - including `ffmpeg` and `ffprobe`
- https://github.com/smacke/ffsubsync
- https://mkvtoolnix.download/downloads.html

# Usage

```
$ subbub -h
Usage: subbub.exe [OPTIONS] <COMMAND>

Commands:
  subtitles  commands to modify subtitles
  debug      command for testing
  help       Print this message or the help of the given subcommand(s)

Options:
  -l, --log-level <LOG_LEVEL>  overrides the log level [default: WARN]
  -k, --keep-tmp-files         when specified, keeps temporary files around
  -h, --help                   Print help
  -V, --version                Print version
```

```
$ subbub subtitles -h
commands to modify subtitles

Usage: subbub.exe subtitles [OPTIONS] --input <INPUT> --output <OUTPUT> <COMMAND>

Commands:
  convert-subtitles  converts the given subtitle file(s) to srt format
  strip-html         strips html from the given subtitle file(s)
  shift-timing       shifts the timing of the given subtitle(s) earlier or later by the given value in seconds
  sync               syncs the timing of the given subtitles(s) to the secondary subtitle(s)
  combine            combines the given subtitles with another set of subtitles, creating dual subtitles (displaying both at the same time) primary subtitles will be displayed below the video secondary subtitles will be displayed above the video
  match-videos       takes the subtitles from their current directory and places them alongside the videos present in the output directory also renames them to match the videos this makes the subtitles discoverable by various media library management applications
  add-subtitles      adds given subtitle(s) (-i/--input) to the given video(s) (-v/--video_path)
  help               Print this message or the help of the given subcommand(s)

Options:
  -i, --input <INPUT>    the subtitles used as input
                         this may be a subtitles file, a video file, or a directory containing either subtitles files or video files
  -t, --track <TRACK>    the subtitles track to use if the input is a video
  -o, --output <OUTPUT>  the location to output the modified subtitles
                         if the input contains multiple subtitles, this will be considered a directory, otherwise, a filename
  -h, --help             Print help
  -V, --version          Print version
```

# Examples

## Sync

Sync subs in the folder `subs` with the subs on track 0 of videos in the current working directory, outputting the synced subs to `synced/`
`subbub subtitles -i ./subs -o ./synced sync -r ./ -y 0`

## Combine

Combine subs in the folder `ja` with the subs on track 1 of videos in current working directory, outputting them to `dual-ja/`
`subbub subtitles -i ./ja -o ./dual-ja combine -s ./ -y 1`

## Match

Match subs in the folder `subs/` with the videos in the current working directory, moving the subs next to the videos
`subbub subtitles -i ./subs -o ./ match-videos`
