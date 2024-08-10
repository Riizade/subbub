# Todo

- refactor input/output to be groups of arguments that are mutually exclusive; e.g., you specify a video file's sub track, a subtitle file, or a directory, but not more than one (currently you specify _whatever_ and it just tries to do the right thing)
- add alternative subtitle syncing options
  - shift forward/back by h:m:s:ms
  - sync to audio
  - sync to image subtitle file (PGS, etc)
- update arguments to allow selecting _exactly one_ of the subtitle syncing options
