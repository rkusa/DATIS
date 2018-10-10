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

## Installation

Copy (or link) `lua/datis-hook.lua` to your `Saved Games\DCS.openbeta\Scripts\Hooks\` directory.

Build DATIS with rust nightly:

```
cargo build --release
```

Copy `target/release/datis.dll` to `Saved Games\DCS.openbeta\Scripts\DATIS\`.

Once you start a mission that contains a pattern in the mission situation as described above, DATIS should work automatically (expects a SRS server to run locally on the default SRS ports).
