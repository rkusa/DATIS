# DATIS

DCS World Automatic Terminal Information Service (ATIS) broadcasted though [Simple Radio Standalone](https://github.com/ciribob/DCS-SimpleRadioStandalone).

**Features:** Automatically starts within DCS, extracts weather information from the currently running mission, generates an ATIS report, converts it to speech (using Google's Text-to-Speech cloud service) and broadcasts it through SRS.

[Changelog](./CHANGELOG.md) | [Prebuild Releases](https://github.com/rkusa/DATIS/releases)

Example Report:

> This is Batumi information Alpha.
> Runway in use is 3 1.
> Wind ZERO 4 1 at 8 knots.
> Visibility 4 3.
> Cloud conditions scattered ZERO, rain.
> Temperature 2 3 celcius.
> ALTIMETER 2 NINER 5 3.
> REMARKS 1 ZERO ZERO ZERO hectopascal.
> End information Alpha.

## Sub-Projects

- [**datis-core**](./datis-core) - The core functionality: generating the report and talking to SRS.
- [**datis-cmd**](./datis-cmd) - A utility to start DATIS from the command line. Mostly intended for testing-purposes.
- [**datis-module**](./datis-module) - A Lua module that can be integrated into DCS to automatically start ATIS stations.

## Installation

_Before you start, be aware that this mod requires a Google Cloud account._

Either [build it](#build) yourself and use the content inside the `mod` directory or use the prebuild mod from one of the [releases](https://github.com/rkusa/DATIS/releases).

1. Copy the content (`Mods` and `Scripts` directory) into `Saved Games\DCS.openbeta\`.
2. Go to https://console.cloud.google.com/apis/credentials and create an API key and restrict API access to Google Text-to-Speech
3. Open DCS go to OPTIONS -> SPECIAL -> DCS ATIS, enter the API key into "Google Cloud Access Key" textfield and save

Once you start a mission that contains a pattern as described in the next section, DATIS runs automatically (expects a SRS server to run locally on the default SRS ports).

If the ATIS is not working, you might find some helpful information in the log file at `DCS.openbeta\Logs\DATIS.log`.

## Mission Setup

There are two methods to add an ATIS station. The first method is easier to setup, but the second one will get more configuration options in the future.

### Mission Situation

The first method is to add the following text pattern as often to the mission situation as you like.

```
ATIS {Airfield} {ATIS Frequency}
```

Examples:

```
ATIS Kutaisi 251.000
ATIS Batumi 131.5
ATIS Senaki-Kolkhi 145
```

If you want the ATIS station's report to also include a traffic frequency, add it with the following pattern to the mission briefing as well.

```
TRAFFIC {Airfield} {Traffic Frequency}
```

### Using Static Units

The second method is to add one static unit per ATIS station to the mission. A could fit could be something like a communication tower. The key to get ATIS working, is to name the static unit using the following pattern:

(`{}` denotes a part that has to be replaced with a proper value and `[]` denotes an optional part)

```
ATIS {Airfield} {ATIS Frequency}[, TRAFFIC {TRAFFIC Frequency}][, VOICE {VOICE NAME}]
```

Available voices are: `en-US-Standard-B`, `en-US-Standard-C` (current default), `en-US-Standard-D`, `en-US-Standard-E`, `en-US-Wavenet-A`, `en-US-Wavenet-B`, `en-US-Wavenet-C`, `en-US-Wavenet-D`, `en-US-Wavenet-E`, `en-US-Wavenet-F` _(a bit down [on this page](https://cloud.google.com/text-to-speech/) is a widget where the different voices can easily be tested)_

Keep in mind that `en-US-Wavenet-*` voices come with a smaller free quota (see [Gcloud TTS pricing](https://cloud.google.com/text-to-speech/pricing)).

Examples:

```
ATIS Batumi 131.5
ATIS Kutaisi 251.000, TRAFFIC 252.000
ATIS Kutaisi 251.000, VOICE en-US-Standard-E
ATIS Kutaisi 251.000, TRAFFIC 252.000, VOICE en-US-Standard-E
```

![Example](./docs/static.jpg)

## Build

Instead of building you can also use the prebuild mod from one of the [releases](https://github.com/rkusa/DATIS/releases).

Build with [Rust (beta)](https://rustup.rs/):

```
make release
```

## License

[MIT](./LICENSE.md)
