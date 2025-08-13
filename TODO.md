# Todo

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

## Notes

- alternative sync solutions
  - ffsubsync with no reference attempts to use audio
  - https://github.com/oseiskar/autosubsync
  - https://github.com/kaegi/alass
  - https://github.com/sc0ty/subsync
  - https://github.com/oseiskar/autosubsync
  - https://github.com/pums974/srtsync
