# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

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
