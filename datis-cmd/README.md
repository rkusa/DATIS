# DATIS Command Line Utility

A utility to start DATIS from the command line. Mostly intended for testing-purposes (to not have to restart DCS each time making a change during development).

## Usage

```
USAGE:
    datis-cmd.exe [OPTIONS] --gcloud <gcloud_key>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -f, --freq <frequency>       Sets the SRS frequency (in Hz, e.g. 255000000 for 255MHz) [default: 255000000]
        --gcloud <gcloud_key>     [env: GCLOUD_KEY=]
```
