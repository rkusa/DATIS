# DCS ATIS (DATIS)

To add ATIS stations to your mission, include the following pattern somewhere in your mission briefing:

```
ATIS [Airfield] [ATIS Frequency]
```

Examples:

```
ATIS Kutaisi 251.000
ATIS Batumi 131.5
ATIS Senaki-Kolkhi 145
```

If you want the ATIS station's report to also include a traffic frequency, add it with the following pattern to the mission briefing as well.

```
TRAFFIC [Airfield] [Traffic Frequency]
```

## Build

Requirements:
- Rust nightly

Run

```
make release
```

## Installation

Either use build it yourself and use the content inside the `mod` directory or use the prebuild mod from one of the [releases](https://github.com/rkusa/DATIS/releases).

1. Copy the content (`Mods` and `Scripts` directory) into `Saved Games\DCS.openbeta\`.
2. Go to https://console.cloud.google.com/apis/credentials and create an API key and restrict API access to Google Text-to-Speech
3. Open DCS go to OPTIONS -> SPECIAL -> DCS ATIS, enter the API key into "Google Cloud Access Key" textfield and save

Once you start a mission that contains a pattern in the mission situation as described above, DATIS should work automatically (expects a SRS server to run locally on the default SRS ports).
