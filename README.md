# DATIS

DCS World Automatic Terminal Information Service (ATIS) broadcasted through [Simple Radio Standalone](https://github.com/ciribob/DCS-SimpleRadioStandalone). Automatically starts within DCS, extracts weather information from the currently running mission, generates various report, converts it to speech and broadcasts it through SRS.

[Changelog](./CHANGELOG.md) | [Prebuild Releases](https://github.com/rkusa/DATIS/releases)

Example Report:

> This is Batumi information Alpha. Runway in use is 13. Wind 140 at 9 knots. Visibility 0.5. Cloud conditions overcast 5, rain. Temperature 20.9 celcius. ALTIMETER 2933. REMARKS. 993 hectopascal. QFE 2930 or 992. End information Alpha.

Example Carrier Report:

> 99, Mother's wind 140 at 9 knots, altimeter 2933, CASE 1, BRC 276, expected final heading 267, report initial

## Features

ATIS stations are not the only report kind this mod supports, all supported report kinds are:

- **ATIS stations**: Automatic Terminal Information Service broadcasts.
- **Carrier reports**: Report information relevant to carrier recoveries.
- **Broadcast of custom message**: Convert a custom message to speech and broadcast it through SRS.
- **Weather stations**: Similar report to ATIS stations, but not connected to an airfield. Can be used to report weather conditions at various parts of the map, like shooting ranges.

The mods uses the Window' built-in TTS by default, but can also either use Google Cloud's TTS, AWS Polly, or a combination of all of them (tip: setup both Google Cloud and Amazon Web Services to have a greater choice of different voices).

## Migrate to 3.x.x

The plugin settings have been removed from the DCS settings screen. DATIS is now configured through a config file at `Saved Games\DCS.openbeta\Config\DATIS.json`, see [Settings](./docs/Settings.md).
This has been done, because each mission you save contains all your plugin settings. This included the cloud provider access keys set for DATIS. The change has been made to prevent the accidential sharing of those credentials.

## Installation

[Build it](#build) yourself and use the content inside the `mod` directory or use the pre-build mod from one of the [releases](https://github.com/rkusa/DATIS/releases).

1. Copy the content (`Mods` and `Scripts` directory) into `Saved Games\DCS.openbeta\`.
2. Optional: Customize your settings, see [Settings](./docs/Settings.md)

Once you start a mission that contains a pattern as described in the next section, DATIS runs automatically.
It thereby expects a SRS server to run locally on the default SRS ports.

If DATIS isn't working, you might find some helpful information in the log file at `DCS.openbeta\Logs\DATIS.log`.

For information about the free tier of both GCloud and AWS see:
- https://cloud.google.com/text-to-speech/pricing
- https://aws.amazon.com/polly/pricing/

## Setup

### Setup ATIS stations

ATIS stations can be added to your mission by either
- adding the following pattern to your mission situation, or
- by adding a static unit to your mission, e.g. a communication tower (the kind of static unit doesn't matter though), and name the static unit using the following pattern.


```
ATIS {Airfield} {ATIS Frequency}[, OPTION {VALUE}]*
```

Available settings:

- `VOICE {VOICE NAME}`: Set the TTS provider and voice to be used for this station. If not provided, the TTS provider and voice defaults to the one set up in the DCS special settings. Available voices are:
  - Windows: `WIN` or `WIN:voice`: Available voice names are: `Catherine` (en-AU), `James` (en-AU), `Linda` (en-CA), `Richard` (en-CA), `George` (en-GB), `Hazel` (en-GB), `Susan` (en-GB), `Sean` (en-IE), `Heera` (en-IN), `Ravi` (en-IN), `David` (en-US), `Zira` (en-US), `Mark` (en-US). Make sure  to install the corresponding voice package (for the language of the voice) for the voices to be available.
  - Google Gloud (`GC:{VOICE NAME}`): For available voices see https://cloud.google.com/text-to-speech/docs/voices. Use the name from the `Voice name` column. All voices starting with `en-` are supported. Keep in mind that `en-US-Wavenet-*` voices come with a smaller free quota, see [Gcloud TTS pricing](https://cloud.google.com/text-to-speech/pricing).
  - AWS (`AWS:{VOICE NAME}`): For available voices see https://docs.aws.amazon.com/polly/latest/dg/voicelist.html. Use the name from the `Name/ID` column (without `*` prefixes). All English voices are supported.
- `TRAFFIC {FREQUENCY}`: An optional traffic frequency that, if provided, is mentioned as part of the ATIS report.
- `OVERRIDE {INFO LETTER}`: Allows you to override the dynamic rotating selection of the ATIS information letter if your mission requires a specific and constant value.
- `ACTIVE {RUNWAY OVERRIDE}`: Can be used if the SPINS for the airfield differ from the prevailing winds and you want to override the calculated active runway.
- `QNH {QNH OVERRIDE in inHg}`: Can be used if the QNH announcement needs to be a specific value, usually to include compensation for temperature which DCS's data export does not include.
- `NO HPA`: Disable adding pressures in hectopascals to the remarks section..
- `NO QFE`: Disabled inclusion of QFE in the remarks section.

Examples:

```
ATIS Kutaisi 251.000
ATIS Batumi 131.5
ATIS Senaki-Kolkhi 145
ATIS Kutaisi 251.000, TRAFFIC 252.000
ATIS Kutaisi 251.000, VOICE en-US-Standard-E
ATIS Kutaisi 251.000, TRAFFIC 252.000, VOICE en-US-Standard-E
ATIS Kutaisi 251.000, TRAFFIC 252.000, VOICE GC:en-US-Wavenet-B
ATIS Kutaisi 251.000, TRAFFIC 252.000, VOICE AWS:Nicole
ATIS Kutaisi 251.000, TRAFFIC 252.000, VOICE WIN
ATIS Kutaisi 251.000, TRAFFIC 252.000, INFO Q, ACTIVE 21L, QNH 30.02, NO QFE
```

![Example](./docs/static.jpg)

### Setup Carrier Reports

Name your carrier unit (unit not group!) using the following pattern:

```
CARRIER {Name} {Frequency}[, VOICE {VOICE NAME}]
```

![Example](./docs/carrier.jpg)

### Setup Broadcast of Custom Messages

Place a unit (doesn't matter if it is a static unit, a plane, a vehicle, ...) and name it (the unit not the group!) using the following pattern:

```
BROADCAST {Frequency}[, VOICE {VOICE NAME}]: {Message}
```

Example:

```
BROADCAST 251.000, VOICE AWS:Brian: Help help!
```

### Setup Broadcast of Weather Stations

Place a unit (doesn't matter if it is a static unit, a plane, a vehicle, ...) and name it (the unit not the group!) using the following pattern:

```
WEATHER {Station Name} {Frequency}[, VOICE {VOICE NAME}]
```

Example:

```
WEATHER Mountain Range 251.000, VOICE en-US-Standard-E
```

## Development

### Crates

- [**datis-cmd**](./crates/datis-cmd) - A utility to start DATIS from the command line. Mostly intended for testing-purposes.
- [**datis-core**](./crates/datis-core) - The core functionality: generating the report and talking to SRS.
- [**datis-module**](./crates/datis-module) - A Lua module that can be integrated into DCS to automatically start ATIS stations.
- [**radio-station**](./crates/datis-station) - A command line utility to play OGG/OPUS audio files through a specified SRS frequency.
- [**srs**](./crates/srs) - A re-usable Rust SRS client that is used for all the other crates.
- [**win-media**](./crates/win-media) - Bindings to a subset of the Windows Runtime (separate crate to reduce the compile time during development).
- [**win-tts**](./crates/win-tts) - A library to convert text to speech using the Window Runtime.

### Build

Instead of building you can also use the pre-build mod from one of the [releases](https://github.com/rkusa/DATIS/releases).
Otherwise, build with [Rust (stable)](https://rustup.rs/):

```
make release
```

### Run tests

```bash
make test
```

### Format code

```bash
make fmt
```

This requires you to have `rustfmt` on your toolchain. It can be installed via `rustup component add rustfmt`.

## License

[MIT](./LICENSE.md)
