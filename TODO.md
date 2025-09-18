# Todo

## Bugs
- for add-dual-subs, if output folder does not already exist, fails with "file does not exist"

## Features
- add alternative subtitle syncing options
  - shift forward/back by h:m:s:ms
  - sync to audio
  - sync to image subtitle file (PGS, etc)
- update arguments to allow selecting _exactly one_ of the subtitle syncing options
- split add-dual-subs into add-synced-subs and add-dual-subs
- add command to do both of the above (add-and-combine-subs?)
- create subtitles from audio using whisper ([which has been added to ffmpeg](https://news.ycombinator.com/item?id=44886647))
- translate subtitles using DeepL or similar
  - supply API key as a CLI argument
  - because subtitles between e.g., Japanese and English are structured differently
    - first merge all subtitles less than say, 500ms apart (showed on-screen consecutively because they're part of the same line)
    - then translate these merged subtitles
    - now the translated subtitles are too long, so split them again into smaller chunks
    - this will not work well if multiple characters are talking
- a "release" github action that auto-builds binaries for Windows, macOS, and Linux and creates a release page

## Notes

- alternative sync solutions
  - ffsubsync with no reference attempts to use audio
  - https://github.com/oseiskar/autosubsync
  - https://github.com/kaegi/alass
  - https://github.com/sc0ty/subsync
  - https://github.com/oseiskar/autosubsync
  - https://github.com/pums974/srtsync
