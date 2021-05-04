# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [3.0.1] - 2021-05-02

### Added

- Added a 500ms break between the two different QFE reports (`QFE 2997 <break time=\"500ms\" /> or 1015.`) [#83](https://github.com/rkusa/DATIS/issues/83)

### Fixed

- Fixed QNH and QFE for temperatures other than 16°C [#85](https://github.com/rkusa/DATIS/issues/85)

### Changed

- Made the internal handling of different measurement units (like foot vs meter and pascal vs inHg) type-safe which should help to prevent measurement conversion-related errors in the future.

## [3.0.0] - 2021-04-25

The following list is a summary of changes of all previous beta releases, there were no changes since the previous beta `3.0.0-beta.3`,

### Added

- **BREAKING:** Added support for DCS 2.7 cloud presets. Requires DCS 2.7 from now on.
- Added Azure as an additional TTS provider [#90](https://github.com/rkusa/DATIS/pull/90) (thanks [ugene143](https://github.com/ugene143) for the implementation!)

### Removed

- **BREAKING:** The plugin settings have been removed from the DCS settings screen. DATIS is now configured through a config file at `Saved Games\DCS.openbeta\Config\DATIS.json`, see [Settings](./docs/Settings.md). Existing settings are not automatically migrated. This has been done, because each mission you save contains all your plugin settings. This included the cloud provider access keys set for DATIS. The change has been made to prevent the accidential sharing of those credentials.

## [3.0.0-beta.3] - 2021-04-22

### Added

- Added Azure as an additional TTS provider [#90](https://github.com/rkusa/DATIS/pull/90) (thanks [ugene143](https://github.com/ugene143) for the implementation!)

## [3.0.0-beta.2] - 2021-04-20

### Removed

- **BREAKING:** The plugin settings have been removed from the DCS settings screen. DATIS is now configured through a config file at `Saved Games\DCS.openbeta\Config\DATIS.json`, see [Settings](./docs/Settings.md). Existing settings are not automatically migrated. This has been done, because each mission you save contains all your plugin settings. This included the cloud provider access keys set for DATIS. The change has been made to prevent the accidential sharing of those credentials.

## [3.0.0-beta.1] - 2021-04-20

### Added

- **BREAKING:** Added support for DCS 2.7 cloud presets. Requires DCS 2.7 from now on.

## [2.2.2] - 2021-04-11

### Changed

- Changed the wind direction of ATIS reports from true to magnetic north

### Fixed

- Correctly report wind speed at carrier in feet (instead of in m/s) [#99](https://github.com/rkusa/DATIS/issues/88)

## [2.2.1] - 2021-01-24

### Fixed

- Fixed pronunciation pronunciation of noun "wind" (from `/waɪnd/` to `/wɪnd/`) for Windows' TTS. [#80](https://github.com/rkusa/DATIS/issues/80)
- Fixed carrier report to report correct magnetic BRC (by estimating the magnetic declination using the IGRF model). [#68](https://github.com/rkusa/DATIS/issues/68)

## [2.2.0] - 2021-01-22

The following list is a summary of changes of all previous beta releases, there were no changes since the previous beta `2.2.0-beta.7`,

### Added
- Added a new option to override the information later that an ATIS station starts with. Thanks a lot [@talbotmcinnis](https://github.com/talbotmcinnis) for the implementation! [#69](https://github.com/rkusa/DATIS/pull/69)
- Added support for all non-US (en_GB, en_AU, ...) English Google Cloud voices
- Added support for new English AWS (Polly) voices
- Added all possible Windows TTS English voices to DATIS default voice setting dropdown

### Changed
- It is now possible to set station options in any order and you'll now receive a useful error message for most kind of typos in the configuration, instead of that the station does simply not work. Thanks a lot [@talbotmcinnis](https://github.com/talbotmcinnis) for the implementation! [#73](https://github.com/rkusa/DATIS/pull/73)
- ATIS stations setup via the misison situation can now use all additional settings that were previously only available to the static unit setup method. Thanks a lot [@talbotmcinnis](https://github.com/talbotmcinnis) for the implementation! [#78](https://github.com/rkusa/DATIS/pull/78)
- Major version upgrades of internal libraries
- Rewrote the Lua module based on the excelent [mlua](https://github.com/khvzak/mlua) library. This removes all `unsafe` code from DATIS and allows to dynamically link against DCS' Lua dll instead of statically linking against Lua (which is discouraged for Lua modules).
- Rewrote the RPC between DATIS and DCS to directly serialize/deserialize Lua data structures instead of going through JSON.

### Fixed
- Fixed Windows TTS-based ATIS stations to support non US voices (e.g. en_GB)
- Fixed station deadlock on certain RPC errors
- Fixed RPC methods to not fail when receiving numbers with exponents
- Fixed carrier boradcast to say "ninety-nine" instead of "niner niner" when addressing all personnel dialed to the frequency #66
- Allow using neutral statics and units as ATIS/Weather/Broadcast stations #65
- Fixed BROADCAST stations for WIN and AWS TTS

## [2.2.0-beta.7] - 2021-01-19

### Fixed
- Fixed actually using the voice setting of ATIS stations defined in the mission situation
- Fixed Windows TTS-based ATIS stations to support non US voices (e.g. en_GB)

### Added
- Added support for all non-US (en_GB, en_AU, ...) English Google Cloud voices
- Added support for new English AWS (Polly) voices
- Added all possible Windows TTS English voices to DATIS default voice setting dropdown

## [2.2.0-beta.6] - 2021-01-16

### Added
- Added a new option to override the information later that an ATIS station starts with. Thanks a lot [@talbotmcinnis](https://github.com/talbotmcinnis) for the implementation! [#69](https://github.com/rkusa/DATIS/pull/69)

### Changed
- It is now possible to set station options in any order and you'll now receive a useful error message for most kind of typos in the configuration, instead of that the station does simply not work. Thanks a lot [@talbotmcinnis](https://github.com/talbotmcinnis) for the implementation! [#73](https://github.com/rkusa/DATIS/pull/73)
- ATIS stations setup via the misison situation can now use all additional settings that were previously only available to the static unit setup method. Thanks a lot [@talbotmcinnis](https://github.com/talbotmcinnis) for the implementation! [#78](https://github.com/rkusa/DATIS/pull/78)
- Major version upgrades of internal libraries

## [2.2.0-beta.5] - 2020-11-01

### Added
- The info letter an ATIS station starts at can now be configured (is randomly choosen otherwise). Thanks to @talbotmcinnis #69

### Fixed
- Fixed station deadlock on certain RPC errors
- Fixed RPC methods to not fail when receiving numbers with exponents

## [2.2.0-beta.4] - 2020-09-27

### Fixed
- Fixed carrier boradcast to say "ninety-nine" instead of "niner niner" when addressing all personnel dialed to the frequency #66

## [2.2.0-beta.3] - 2020-09-22

### Fixed
- Allow using neutral statics and units as ATIS/Weather/Broadcast stations #65

## [2.2.0-beta.2] - 2020-09-18

### Fixed
- Fixed extraction of active runways (regression introduced in `2.2.0-beta.1`)
- Fixed all stations after a mission change (regression introduced in `2.2.0-beta.1`)
- Fixed BROADCAST stations for WIN and AWS TTS

## [2.2.0-beta.1] - 2020-09-15

This release includes internal improvements only.

### Changed
- Rewrote the Lua module based on the excelent [mlua](https://github.com/khvzak/mlua) library. This removes all `unsafe` code from DATIS and allows to dynamically link against DCS' Lua dll instead of statically linking against Lua (which is discouraged for Lua modules).
- Rewrote the RPC between DATIS and DCS to directly serialize/deserialize Lua data structures instead of going through JSON.

## [2.1.0] - 2020-07-05

**Manually upgrade step needed:** The mod was moved from `Mods/tech` to `Mods/services`. After upgrading DATIS, please remove the old mod directory `Mods/tech/DATIS` manually.

### Added
- Added `--port` option to radio-station command #52

### Changed
- Frequencies below or equal to `87.995` will now automatically use an `FM` modulation (everything else will continue to be `AM`) #51

### Fixed
- Fixed weather and broadcast stations that are using static units #56
- Fixed DCS not authorized error by moving mod from `Mods/tech` to `Mods/services` #53

## [2.0.0] - 2020-06-20
No changes since `2.0.0-beta.1`.

### Changed
- **Breaking:** Upgraded SRS protocol to 1.9.0.0 (requires SRS server version 1.9.x.x)

## [2.0.0-beta.1] - 2020-06-16
### Changed
- **Breaking:** Upgraded SRS protocol to 1.9.0.0 (requires SRS server version 1.9.x.x)

## [1.1.0] - 2020-05-24
### Changed
- Update shutdown procedure to gracefully shut down all stations instead of killing them

### Fixed
- Fixed Windows' TTS to try use an english voice by default
- Add Windows' TTS related logs to the DATIS.log file

## [1.1.0-rc.1] - 2020-05-17
### Changed
- Changed the internal implementation of how DATIS talks to Windows' TTS (from calling a .NET application to a pure Rust implementation that uses WinRT bindings)

### Added
- Added weather stations. Weather stations are similar to ATIS stations, except that they are not related to any airfield. Their report is pretty much the same, except the active runway and the traffic frequency. To setup a weather station, name a unit with the following pattern `WEATHER {name} {frequency}[, VOICE {voice name}]`. The station will report the weather the unit's position.

### Fixed
- Fixed broadcast stations to work with statics.

## [1.0.0] - 2020-05-13

No changes since the previous pre-release (`1.0.0-beta.3`), the following is just a summary of change since version `0.10.0`.

### Added
- Added default voice option to DATIS settings
- Units can now broadcast custom messages. Just name a unit with the following pattern `BROADCAST {frequency}[, VOICE {voice name}]: {custom message}` and the `{custom message}` will be converted to speech using the voice `{voice name}` and broadcasted over the `{frequency}`.
- Support for Windows' TTS as an additional TTS provider #24
- Possibility to select a voice for Windows' TTS (e.g. `VOICE WIN:David`, or `VOICE WIN:Zira`)
- Carrier ATIS (reports altimeter, BRC and Case variant to be used; eg.: `CARRIER Mother 250.000`)
- Support for AWS Polly as an additional TTS provider (implemented by @16AGR-Durham)
- The internals of DATIS are no based on async Rust. With that, DATIS does not create two threads for each ATIS station anymore (max. number of threads is now the number of cores available to the system).

### Changed
- **Breaking:** Upgraded SRS protocol to 1.8.0.0 (requires SRS server version 1.8.x.x)
- **Breaking:** Made `WIN` the default voice (instead of `GC:en-US-Standard-C`)
- Every unit can now act as an ATIS station (not just statics)
- Every unit can now act as a Carrier station (not just ships; not really a useful feature, just a side-effect)
- Start each airfield station with a random information letter #39
- Reworked how DATIS communicates to the running mission - DATIS now has a way to ask the running mission about updates (this RPC functionality handles one request every 2 seconds)

### Fixed
- Fixed wind speed unit (convert the m/s received from DCS to knots) #40
- In some cases stations stopped transmitting every ~0.5 secs should be fixed
- Restore consistent ~3sec pause between reports
- Fixed DATIS crashing MP servers
- Possibly fixed DATIS not picking up weather updates for missions with dynamic weather #16
- Derive a visibility value from cloud and fog weather settings (instead of using the visibility mission setting that never changes)

## [1.0.0-beta.3] - 2020-05-11
### Changed
- Reduced the frequency in which RPC requests from ATIS stations are handled inside the DCS hook of DATIS to one request every 2 seconds (it was one request every half second before). This change will slow down the initial start of all stations a bit, but shouldn't be noticeable otherwise.

### Fixed
- Fixed default voice for ATIS stations

## [1.0.0-beta.2] - 2020-05-05
### Changed
- Explicitly use a multi-threaded scheduler to run DATIS stations

## [1.0.0-beta.1] - 2020-05-03
### Fixed
- Fix WIN as default voice not working

### Changed
- **Breaking:** Carrier reports must now be prefixed with `CARRIER` instead of `ATIS`, eg.: `CARRIER Mother 250.000`

## [1.0.0-alpha.10] - 2020-05-01
### Added
- Added default voice setting to DATIS settings
- Units can now broadcast custom messages. Just name a unit with the following patter `BROADCAST {frequency}[, VOICE {voice name}]: {custom message}` and the `{custom message}` will be converted to speech using the voice `{voice name}` and broadcasted over the `{frequency}`.

### Changed
- **Breaking:** Made `WIN` the default voice (instead of `GC:en-US-Standard-C`)
- Made `eu-central-1` the default AWS region
- Every unit can now act as an ATIS station (not just statics)
- Every unit can now act as a Carrier station (not just ships; not really a useful feature, just a side-effect)

### Fixed
- Added new SRS 1.8.0.0 radio modulation variants

## [1.0.0-alpha.9] - 2020-04-28
### Changed
- **Breaking:** Upgraded SRS protocol to 1.8.0.0 (requires SRS server version 1.8.x.x)

## [1.0.0-alpha.8] - 2020-04-24
### Fixed
- Fixed `Trying to access undefined lua global or table key: frequency` error on the TTI mission #42

### Changed
- Updated internal dependencies

## [1.0.0-alpha.7] - 2020-03-03
### Changed
- Start each airfield station with a random information letter #39

### Fixed
- Fixed wind speed unit (convert the m/s received from DCS to knots) #40

## [1.0.0-alpha.6] - 2019-11-16
### Fixed
- Update SRS message parser to work with SRS 1.7.0.3

## [1.0.0-alpha.5] - 2019-11-06
### Fixed
- Fix build to not use static test weather reports for each station

## [1.0.0-alpha.4] - 2019-11-02
### Added
- Possibility to select a voice for Windows' TTS (e.g. `VOICE WIN:David`, or `VOICE WIN:Zira`)

### Fixed
- Fixed SRS message decoding error

## [0.10.0] - 2019-11-02
### Changed
- Upgraded to the SRS 1.7.0.0 network changes. DATIS now acts as a 1.7.0.0 SRS client.

### Fixed
- In some cases stations stopped transmitting every ~0.5 secs should be fixed
- Restore consistent ~3sec pause between reports

## [1.0.0-alpha.3] - 2019-10-31
### Fixed
- Fixed DATIS crashing MP servers
- Possibly fixed DATIS not picking up weather updates for missions with dynamic weather #16

### Changed
- Reworked how DATIS communicates to the running mission - DATIS now has a way to ask the running mission about updates

## [1.0.0-alpha.2] - 2019-10-31
### Added
- Support for Windows' TTS as an additional TTS provider #24
- Carrier ATIS (reports altimeter, BRC and Case variant to be used)

### Fixed
- Derive a visibility value from cloud and fog weather settings (instead of using the visibility mission setting that never changes)

## [1.0.0-alpha.1] - 2019-10-25
Moving to `1.0.0` as encouraged by semantic versioning.

### Added
- Support for AWS Polly as an additional TTS provider (implemented by @16AGR-Durham)

### Changed
- Upgrade to the SRS 1.7.0.0 network changes. DATIS now acts as a 1.7.0.0 SRS client.
- The internals of DATIS are no based on async Rust. With that, DATIS does not create two threads for each ATIS station anymore (max. number of threads is now the number of cores available to the system).

## [0.10.0-beta.1] - 2019-10-18
### Changed
- Upgrade to the SRS 1.7.0.0 network changes. DATIS now acts as a 1.7.0.0 SRS client.

## [0.9.2] - 2019-10-28
### Fixed
- In some cases stations stopped transmitting every ~0.5 secs should be fixed
- Restore consistent ~3sec pause between reports

## [0.9.1] - 2019-10-14
### Fixed
- Do not SPAM logs when connection to SRS is lost #18

### Changed
- Automatically try to re-connect to SRS if connection to SRS got lost or if DATIS is started before SRS

## [0.9.0] - 2019-08-13
### Added
- Option to enable debug logging (useful when investigating issues; logs into `Saved Games\DCS\Logs\DATIS.log`)
- Option to change the SRS Server port

### Fixed
- Fix rounding issue in parsing of certain frequencies #15

## [0.8.0] - 2019-04-23
### Changed
- DATIS now acts as an SRS version 1.6.0.0 client (and thus doesn't work with <1.6 servers anymore)

## [0.7.1] - 2019-03-03
### Fixed
- Fix missing `Terrain` global when starting DCS server with `--norender`

### Changed
- Use different log levels for DCS Lua hook

## [0.7.0] - 2019-01-10
### Changed
- Added radio information to the initial SRS sync message #10
- All ATIS reports are now exported into the DCS saved games directory into `Logs\atis-reports.json`

### Added
- Log a warning if there are no ATIS stations found

### Fixed
- Fix parsing of airfield names that contain a space in their name
- Fix wind speed unit

## [0.6.0] - 2018-12-04
### Changed
- Reduced logging output (to started, stopped and error messages)

## [0.5.0] - 2018-11-28
### Added
- Added QFE in inHg and hPa to the ATIS report remarks

### Fixed
- Fixed reading runways with a _L_ or _R_ suffix

## [0.4.4] - 2018-11-21
### Fixed
- Properly rotate and normalize the DCS reported wind direction

## [0.4.3] - 2018-11-11
### Fixed
- Fixed active runway calculation

## [0.4.2] - 2018-11-07
### Fixed
- Pressure and temperature readings should now also work when the server does not hit the "Briefing" button
- SRS broadcasts should now always be stopped when the mission is stopped

## [0.4.1] - 2018-11-04
### Fixed
- Properly handle and report Gcloud TTS API errors (the previous error message was not useful at all, see #8)

## [0.4.0] - 2018-11-03
### Added
- Added option to setup ATIS stations by [adding static units](https://github.com/rkusa/DATIS#mission-setup) with a specific naming scheme to the mission
- Possibility to use a different voice per station

### Fixed
- TTS should now properly read ZERO and not "o"
- Pressure and temperature readings fixed for multiplayer servers (was only working correctly in SP)

### Changed
- Added "Cloud conditions" in front of the cloud report
- Added longer breaks between the different report parts
- Skip the "DECIMAL" when calling out the altimeter setting

## [0.3.0] - 2018-10-24
### Changed
- Extracted Google Cloud Access Key into "DCS ATIS" option specials menu

## [0.2.1] - 2018-10-12
### Fixed
- reverted most phonetic number replacements (TTL handles normal numbers fine)

## [0.2.0] - 2018-10-12
### Added
- visibility report
- clouds report

### Fixed
- QNH properly read at ground level
