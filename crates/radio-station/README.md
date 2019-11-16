# DCS Radio Station

A command line utility to play OGG/OPUS audio files through a specified SRS frequency (expects a SRS server to run locally on the default SRS ports).

## Usage

```
USAGE:
    dcs-radio-station.exe [FLAGS] [OPTIONS] <PATH>

FLAGS:
    -h, --help       Prints help information
    -l, --loop       Enables endlessly looping the audio file(s)
    -V, --version    Prints version information

OPTIONS:
    -f, --freq <frequency>    Sets the SRS frequency (in Hz, e.g. 255000000 for 255MHz) [default: 255000000]

ARGS:
    <PATH>    Sets the path audio file(s) should be read from
```

## Build

Build with [Rust stable](https://rustup.rs/):

```
cd .\drs-cmd
cargo build --release
```

Broadcast audio files (ogg/opus) to DCS World's [Simple Radio Standalone](https://github.com/ciribob/DCS-SimpleRadioStandalone).

To run it during development:

```
cargo run --bin dcs-radio-station -- .\crates\radio-station\example\
```

## Audio Format

**Audio files have to be of the format OGG/OPUS (not OGG/VORBIS)!**

Instructions to convert audio files to OGG/OPUS:
- [using VLC](./docs/convert-with-vlc.md)

