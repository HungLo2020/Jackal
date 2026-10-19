# Optional audio asset export

The game plays the checked-in PCM WAV files. Python, reference execution, and the
original program bytes are not needed to build or run it.

To reproduce all 43 WAV files and their loop metadata without changing assets:

```sh
python3 tools/reference/export_audio.py --check
```

Omit `--check` to replace the audio assets. `--output /tmp/audio-assets` exports to
another directory. Dependencies are pinned by the historical Cargo.lock; add
`--online` if those versions are not already cached.

The exporter retrieves the reference implementation and program data from local
Git commit `e8fc3391d5b27fcd98b82fd27984141840449708` into a temporary directory.
It runs each top-level sound sequence through the recovered driver and five-channel
synthesizer, including original DMC samples. Complete sequencer-state repetition
identifies loop boundaries; finite effects include a short release tail.
The support Rust files contain export entry points only. Legacy source and data
are never copied into the native game's source or runtime assets.

Waveforms reflect the reference synthesizer's frame timing and analog-filter
approximation. This is a deterministic asset export, not an analog recording from
original hardware. The manifest records sample counts, command provenance, and
loop boundaries. No timestamps are generated.
