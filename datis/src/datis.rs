use std::cell::RefCell;
use std::str::FromStr;
use std::thread;

use crate::station::{Airfield, AtisStation, FinalStation, Position, StaticWind};
use crate::utils::create_lua_state;
use hlua51::{Lua, LuaFunction, LuaTable};
use regex::Regex;

type LuaError = usize;

pub struct Datis {
    pub stations: Vec<AtisStation>,
}

impl Datis {
    pub fn create(mut lua: Lua<'_>, cpath: String) -> Result<Self, LuaError> {
        debug!("Extracting ATIS stations from Mission Situation");

        let mut stations = {
            // TODO: unwrap
            let mut dcs: LuaTable<_> = lua.get("DCS").unwrap();

            // TODO: unwrap
            let mut get_mission_description: LuaFunction<_> =
                dcs.get("getMissionDescription").unwrap();
            let mission_situation: String = get_mission_description.call().unwrap();

            extract_atis_stations(&mission_situation)
        };

        debug!("Detected ATIS Stations:");
        for station in &stations {
            debug!("  - {} (Freq: {})", station.name, station.atis_freq);
        }

        // FETCH AIRDROMES
        {
            // TODO: unwrap
            let mut terrain: LuaTable<_> = lua.get("Terrain").unwrap();

            // TODO: unwrap
            let mut get_terrain_config: LuaFunction<_> = terrain.get("GetTerrainConfig").unwrap();
            let mut airdromes: LuaTable<_> =
                get_terrain_config.call_with_args("Airdromes").unwrap();

            let mut i = 12;
            while let Some(mut airdrome) = airdromes.get::<LuaTable<_>, _, _>(i) {
                i += 1;

                // TODO: unwrap
                let id: String = airdrome.get("id").unwrap();
                let display_name: String = airdrome.get("display_name").unwrap();

                for station in stations.iter_mut() {
                    if station.name != id && station.name != display_name {
                        continue;
                    }

                    station.name = display_name.to_string();

                    let (x, y) = {
                        // TODO: unwrap
                        let mut reference_point: LuaTable<_> =
                            airdrome.get("reference_point").unwrap();
                        let x: f64 = reference_point.get("x").unwrap();
                        let y: f64 = reference_point.get("y").unwrap();
                        (x, y)
                    };

                    let alt = {
                        // TODO: unwrap
                        let mut default_camera_position: LuaTable<_> =
                            airdrome.get("default_camera_position").unwrap();
                        let mut pnt: LuaTable<_> = default_camera_position.get("pnt").unwrap();
                        let alt: f64 = pnt.get(2).unwrap();
                        // This is only the alt of the camera position of the airfield, which seems to be
                        // usually elevated by about 100. Keep the 100 elevation above the ground
                        // as a sender position (for SRS LOS).
                        alt
                    };

                    // TODO: unwrap
                    let mut rwys: Vec<String> = Vec::new();

                    let mut runways: LuaTable<_> = airdrome.get("runways").unwrap();
                    let mut j = 0;
                    while let Some(mut runway) = runways.get::<LuaTable<_>, _, _>(j) {
                        j += 1;
                        // TODO: unwrap
                        let start: String = runway.get("start").unwrap();
                        let end: String = runway.get("end").unwrap();
                        rwys.push(start);
                        rwys.push(end);
                    }

                    station.airfield = Some(Airfield {
                        position: Position { x, y, alt },
                        runways: rwys,
                    });

                    break;
                }
            }
        }

        stations.retain(|s| s.airfield.is_some());

        // get _current_mission.mission.weather
        // TODO: unwrap
        let mut current_mission: LuaTable<_> = lua.get("_current_mission").unwrap();
        let mut mission: LuaTable<_> = current_mission.get("mission").unwrap();
        let mut weather: LuaTable<_> = mission.get("weather").unwrap();

        // get atmosphere_type
        // TODO: unwrap
        let atmosphere_type: f64 = weather.get("atmosphere_type").unwrap();

        if atmosphere_type == 0.0 {
            // is static DCS weather system
            // get wind
            // TODO: unwrap
            let mut wind: LuaTable<_> = weather.get("wind").unwrap();
            let mut wind_at_ground: LuaTable<_> = wind.get("wind_at_ground").unwrap();

            // get wind_at_ground.speed
            // TODO: unwrap
            let wind_speed: f64 = wind_at_ground.get("speed").unwrap();

            // get wind_at_ground.dir
            // TODO: unwrap
            let mut wind_dir: f64 = wind_at_ground.get("dir").unwrap();

            for station in stations.iter_mut() {
                // rotate dir
                wind_dir -= 180.0;
                if wind_dir < 0.0 {
                    wind_dir += 360.0;
                }

                station.static_wind = Some(StaticWind {
                    dir: wind_dir.to_radians(),
                    speed: wind_speed,
                });
            }
        }

        debug!("Valid ATIS Stations:");
        for station in &stations {
            debug!("  - {} (Freq: {})", station.name, station.atis_freq);
        }

        for station in stations {
            let cpath = cpath.clone();
            thread::spawn(move || {
                let airfield = station.airfield.as_ref().unwrap();
                let code = format!(
                    r#"
                    local Weather = require 'Weather'
                    local position = {{
                        x = {},
                        y = {},
                        z = {},
                    }}

                    getWeather = function()
                        local wind = Weather.getGroundWindAtPoint({{ position = position }})
				        local temp, pressure = Weather.getTemperatureAndPressureAtPoint({{
				            position = position
				        }})

				        return {{
				            windSpeed = wind.v,
				            windDir = wind.a,
				            temp = temp,
				            pressure = pressure,
				        }}
                    end
                "#,
                    airfield.position.x, airfield.position.alt, airfield.position.y,
                );
                debug!("Loading Lua: {}", code);

                // TODO: unwrap
                let new_state = create_lua_state(&cpath, &code).unwrap();
                let station = FinalStation {
                    name: station.name,
                    atis_freq: station.atis_freq,
                    traffic_freq: station.traffic_freq,
                    airfield: station.airfield.unwrap(),
                    static_wind: station.static_wind,
                    state: RefCell::new(new_state),
                };

                if let Err(err) = crate::srs::start(station) {
                    error!("{}", err);
                }
            });
        }

        Ok(Datis {
            stations: Vec::new(),
        })
    }
}

fn extract_atis_stations(situation: &str) -> Vec<AtisStation> {
    // extract ATIS stations and frequencies
    let re = Regex::new(r"ATIS ([a-zA-Z-]+) ([1-3]\d{2}(\.\d{1,3})?)").unwrap();
    let mut stations: Vec<AtisStation> = re
        .captures_iter(situation)
        .map(|caps| {
            let name = caps.get(1).unwrap().as_str().to_string();
            let freq = caps.get(2).unwrap().as_str();
            let freq = (f32::from_str(freq).unwrap() * 1_000_000.0) as u64;
            AtisStation {
                name,
                atis_freq: freq,
                traffic_freq: None,
                airfield: None,
                static_wind: None,
            }
        })
        .collect();

    // extract optional traffic frquencies
    let re = Regex::new(r"TRAFFIC ([a-zA-Z-]+) ([1-3]\d{2}(\.\d{1,3})?)").unwrap();
    for caps in re.captures_iter(situation) {
        let name = caps.get(1).unwrap().as_str().to_string();
        let freq = caps.get(2).unwrap().as_str();
        let freq = (f32::from_str(freq).unwrap() * 1_000_000.0) as u64;

        if let Some(station) = stations.iter_mut().find(|s| s.name == name) {
            station.traffic_freq = Some(freq);
        }
    }

    stations
}

#[cfg(test)]
mod test {
    use super::{extract_atis_stations, AtisStation};

    #[test]
    fn test_atis_extraction() {
        let stations = extract_atis_stations(
            r#"
            ATIS Kutaisi 251.000
            ATIS Batumi 131.5
            ATIS Senaki-Kolkhi 145

            TRAFFIC Batumi 255.00
        "#,
        );

        assert_eq!(
            stations,
            vec![
                AtisStation {
                    name: "Kutaisi".to_string(),
                    atis_freq: 251_000_000,
                    traffic_freq: None,
                    airfield: None,
                    static_wind: None,
                },
                AtisStation {
                    name: "Batumi".to_string(),
                    atis_freq: 131_500_000,
                    traffic_freq: Some(255_000_000),
                    airfield: None,
                    static_wind: None,
                },
                AtisStation {
                    name: "Senaki-Kolkhi".to_string(),
                    atis_freq: 145_000_000,
                    traffic_freq: None,
                    airfield: None,
                    static_wind: None,
                }
            ]
        );
    }
}
