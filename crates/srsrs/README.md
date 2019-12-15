# DATIS Command Line Utility

A utility to start DATIS from the command line. Mostly intended for testing-purposes (to not have to restart DCS each time making a change during development).

## Usage

```
USAGE:
    datis-cmd.exe [OPTIONS] --tts <tts>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --aws-key <aws_key>           [env: AWS_ACCESS_KEY_ID=]
        --aws-region <aws_region>     [env: AWS_REGION=]  [default: EuCentral1]
    -f, --freq <frequency>           Sets the SRS frequency (in Hz, e.g. 251000000 for 251MHz) [default: 251000000]
        --gcloud <gcloud_key>         [env: GCLOUD_KEY=]
        --tts <tts>                  Sets the TTS provider and voice to be used [default: GC:en-US-Standard-C]
        --aws-secret <was_secret>     [env: AWS_SECRET_ACCESS_KEY=]
```
