# A rewrite of the musiclight2 program in Rust.

A [WLED](https://github.com/Aircoookie/WLED) compatible fork of <https://git.tkolb.de/Public/rust_musiclight>.

## Changes

- Modify UDP protocol to be WLED-compatible.
- Fix bug in band energy calculation
- Decouple racer brightness from racer speed; change settings
- Add bad spectrum visualization (do not use)

## Usage

```
parec --raw --latency=512  --rate 48000 --format s16 --channels 1 -d alsa_output.pci-0000_01_00.1.hdmi-stereo-extra3.monitor | cargo run
```

Or just use `run_pa.sh`.
