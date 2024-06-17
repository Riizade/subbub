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
- Sync subs in the folder `subs/` to track 1 of the subs in the current working directory, then combine them with that track, then match both sets of subs to the videos (my anime command)
  - `export TRACK=1 && subbub subtitles -i ./subs -o ./ja sync -s ./ -y $TRACK && subbub subtitles -i ./ja -o ./dual-ja combine -s ./ -y $TRACK && subbub subtitles -i ./ja -o ./ match-videos && subbub subtitles -i ./dual-ja -o ./ match-videos && rm -rf ./ja && rm -rf ./dual-ja`
