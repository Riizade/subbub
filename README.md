# subbub

A CLI frontend for using mkvextract, ffsubsync, dualsub, etc to manage and merge subtitles

# Requirements

Must have the following installed and available in PATH

- https://ffmpeg.org/
- https://github.com/smacke/ffsubsync

# Examples

- Sync subs in the folder `subs` with the subs on track 0 of videos in the current working directory, outputting the synced subs to `synced/`
  - `subbub subtitles -i ./subs -o ./synced sync -s ./ -y 0`
- Combine subs in the folder `ja` with the subs on track 1 of videos in current working directory, outputting them to `dual-ja/`
  - `subbub subtitles -i ./ja -o ./dual-ja combine -s ./ -y 1`
- Match subs in the folder `subs/` with the videos in the current working directory, moving the subs next to the videos
  - `subbub subtitles -i ./subs -o ./ match-videos`
