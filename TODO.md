# Todo

- refactor input/output to be groups of arguments that are mutually exclusive; e.g., you specify a video file's sub track, a subtitle file, or a directory, but not more than one (currently you specify _whatever_ and it just tries to do the right thing)
- add alternative subtitle syncing options
  - shift forward/back by h:m:s:ms
  - sync to audio
  - sync to image subtitle file (PGS, etc)
- update arguments to allow selecting _exactly one_ of the subtitle syncing options
- split add-dual-subs into add-synced-subs and add-dual-subs
- add command to do both of the above (add-and-combine-subs?)

# Notes

- alternative sync solutions
  - ffsubsync with no reference attempts to use audio
  - https://github.com/oseiskar/autosubsync
  - https://github.com/kaegi/alass
  - https://github.com/sc0ty/subsync
  - https://github.com/oseiskar/autosubsync
  - https://github.com/pums974/srtsync
