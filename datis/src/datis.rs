use std::collections::HashMap;
use std::str::FromStr;

use crate::error::Error;
use crate::srs::AtisSrsClient;
use crate::station::*;
use crate::weather::DynamicWeather;
use hlua51::{Lua, LuaFunction, LuaTable};
use regex::Regex;

pub struct Datis {
    pub clients: Vec<AtisSrsClient>,
}

impl Datis {
    pub fn create(mut lua: Lua<'_>) -> Result<Self, Error> {
        debug!("Extracting ATIS stations from Mission Situation");

        // read `package.cpath`
        let cpath = {
            let mut package: LuaTable<_> = get!(lua, "package")?;
            let cpath: String = get!(package, "cpath")?;
            cpath
        };

        // extract frequencies from mission briefing, which is retrieved from
        // `DCS.getMissionDescription()`
        let frequencies = {
            let mut dcs: LuaTable<_> = get!(lua, "DCS")?;

            let mut get_mission_description: LuaFunction<_> = get!(dcs, "getMissionDescription")?;
            let mission_situation: String = get_mission_description.call()?;

            extract_frequencies(&mission_situation)
        };

        // collect all airfields on the current loaded terrain
        let mut airfields = {
            let mut airfields = HashMap::new();

            // read `Terrain.GetTerrainConfig('Airdromes')`
            let mut terrain: LuaTable<_> = get!(lua, "Terrain")?;
            let mut get_terrain_config: LuaFunction<_> = get!(terrain, "GetTerrainConfig")?;
            let mut airdromes: LuaTable<_> = get_terrain_config.call_with_args("Airdromes")?;

            // on Caucasus, airdromes start at the index 12
            // TODO: check starting index for other maps
            let mut i = 12;
            while let Some(mut airdrome) = airdromes.get::<LuaTable<_>, _, _>(i) {
                i += 1;

                let display_name: String = get!(airdrome, "display_name")?;

                let (x, y) = {
                    let mut reference_point: LuaTable<_> = get!(airdrome, "reference_point")?;
                    let x: f64 = get!(reference_point, "x")?;
                    let y: f64 = get!(reference_point, "y")?;
                    (x, y)
                };

                let alt = {
                    // read `airdrome.default_camera_position.pnt[2]`
                    let mut default_camera_position: LuaTable<_> =
                        get!(airdrome, "default_camera_position")?;
                    let mut pnt: LuaTable<_> = get!(default_camera_position, "pnt")?;
                    let alt: f64 = get!(pnt, 2)?;
                    // This is only the alt of the camera position of the airfield, which seems to be
                    // usually elevated by about 100ft. Keep the 100ft elevation above the ground
                    // as a sender position (for SRS LOS).
                    alt
                };

                let mut runways: Vec<String> = Vec::new();
                let mut rwys: LuaTable<_> = get!(airdrome, "runways")?;
                let mut j = 0;
                while let Some(mut rw) = rwys.get::<LuaTable<_>, _, _>(j) {
                    j += 1;
                    let start: String = get!(rw, "start")?;
                    let end: String = get!(rw, "end")?;
                    runways.push(start);
                    runways.push(end);
                }

                airfields.insert(
                    display_name.clone(),
                    Airfield {
                        name: display_name,
                        position: Position { x, y, alt },
                        runways,
                    },
                );
            }

            airfields
        };

        // read `_current_mission.mission.weather`
        let mut current_mission: LuaTable<_> = get!(lua, "_current_mission")?;
        let mut mission: LuaTable<_> = get!(current_mission, "mission")?;
        let mut weather: LuaTable<_> = get!(mission, "weather")?;

        // read `_current_mission.mission.weather.atmosphere_type`
        let atmosphere_type: f64 = get!(weather, "atmosphere_type")?;

        let static_weather = if atmosphere_type == 0.0 {
            // is static DCS weather system

            let static_wind = {
                // get wind
                let mut wind: LuaTable<_> = get!(weather, "wind")?;
                let mut wind_at_ground: LuaTable<_> = get!(wind, "atGround")?;

                // get wind_at_ground.speed
                let wind_speed: f64 = get!(wind_at_ground, "speed")?;

                // get wind_at_ground.dir
                let mut wind_dir: f64 = get!(wind_at_ground, "dir")?;

                // rotate dir
                wind_dir -= 180.0;
                if wind_dir < 0.0 {
                    wind_dir += 360.0;
                }

                Wind {
                    dir: wind_dir.to_radians(),
                    speed: wind_speed,
                }
            };

            let static_clouds = {
                let mut clouds: LuaTable<_> = get!(weather, "clouds")?;
                Clouds {
                    base: get!(clouds, "base")?,
                    density: get!(clouds, "density")?,
                    thickness: get!(clouds, "thickness")?,
                    iprecptns: get!(clouds, "iprecptns")?,
                }
            };

            let visibility: u32 = {
                let mut visibility: LuaTable<_> = get!(weather, "visibility")?;
                get!(visibility, "distance")?
            };

            Some(Weather {
                wind: static_wind,
                clouds: static_clouds,
                visibility,
            })
        } else {
            None
        };

        let dynamic_weather = DynamicWeather::create(&cpath)?;
        let stations: Vec<Station> = frequencies
            .into_iter()
            .filter_map(|(name, freq)| {
                airfields.remove(&name).and_then(|airfield| {
                    Some(Station {
                        name,
                        atis_freq: freq.atis,
                        traffic_freq: freq.traffic,
                        airfield,
                        static_weather: static_weather.clone(),
                        dynamic_weather: dynamic_weather.clone(),
                    })
                })
            })
            .collect();

        debug!("Valid ATIS Stations:");
        for station in &stations {
            debug!("  - {} (Freq: {})", station.name, station.atis_freq);
        }

        Ok(Datis {
            clients: stations
                .into_iter()
                .map(|station| AtisSrsClient::new(station))
                .collect(),
        })
    }
}

#[derive(Debug, PartialEq)]
struct Frequencies {
    atis: u64,
    traffic: Option<u64>,
}

fn extract_frequencies(situation: &str) -> HashMap<String, Frequencies> {
    // extract ATIS stations and frequencies
    let re = Regex::new(r"ATIS ([a-zA-Z-]+) ([1-3]\d{2}(\.\d{1,3})?)").unwrap();
    let mut stations: HashMap<String, Frequencies> = re
        .captures_iter(situation)
        .map(|caps| {
            let name = caps.get(1).unwrap().as_str().to_string();
            let freq = caps.get(2).unwrap().as_str();
            let freq = (f32::from_str(freq).unwrap() * 1_000_000.0) as u64;
            (
                name,
                Frequencies {
                    atis: freq,
                    traffic: None,
                },
            )
        })
        .collect();

    // extract optional traffic frquencies
    let re = Regex::new(r"TRAFFIC ([a-zA-Z-]+) ([1-3]\d{2}(\.\d{1,3})?)").unwrap();
    for caps in re.captures_iter(situation) {
        let name = caps.get(1).unwrap().as_str();
        let freq = caps.get(2).unwrap().as_str();
        let freq = (f32::from_str(freq).unwrap() * 1_000_000.0) as u64;

        if let Some(freqs) = stations.get_mut(name) {
            freqs.traffic = Some(freq);
        }
    }

    stations
}

#[cfg(test)]
mod test {
    use super::{extract_frequencies, Frequencies};

    #[test]
    fn test_atis_extraction() {
        let freqs = extract_frequencies(
            r#"
            ATIS Kutaisi 251.000
            ATIS Batumi 131.5
            ATIS Senaki-Kolkhi 145

            TRAFFIC Batumi 255.00
        "#,
        );

        assert_eq!(
            freqs,
            vec![
                (
                    "Kutaisi".to_string(),
                    Frequencies {
                        atis: 251_000_000,
                        traffic: None,
                    }
                ),
                (
                    "Batumi".to_string(),
                    Frequencies {
                        atis: 131_500_000,
                        traffic: Some(255_000_000),
                    }
                ),
                (
                    "Senaki-Kolkhi".to_string(),
                    Frequencies {
                        atis: 145_000_000,
                        traffic: None,
                    }
                )
            ]
            .into_iter()
            .collect()
        );
    }
}
