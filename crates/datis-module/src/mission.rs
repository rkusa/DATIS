use std::collections::HashMap;

use datis_core::extract::*;
use datis_core::ipc::*;
use datis_core::station::*;
use datis_core::tts::TextToSpeechProvider;
use mlua::prelude::{Lua, LuaTable, LuaTableExt};
use rand::Rng;

pub struct Info {
    pub stations: Vec<Station>,
    pub ipc: MissionRpc,
}

pub fn extract(lua: &Lua, default_voice: &TextToSpeechProvider) -> Result<Info, mlua::Error> {
    // extract frequencies from mission briefing, which is retrieved from
    // `DCS.getMissionDescription()`
    let station_configs_from_description = {
        let dcs: LuaTable<'_> = lua.globals().get("DCS")?;
        let mission_description: String = dcs.call_function("getMissionDescription", ())?;
        extract_station_config_from_mission_description(&mission_description)
    };

    // Create a random generator for creating the information letter offset.
    let mut rng = rand::thread_rng();

    // collect all airfields on the current loaded terrain
    let mut airfields = {
        let mut airfields = HashMap::new();

        // read `Terrain.GetTerrainConfig('Airdromes')`
        let terrain: LuaTable<'_> = lua.globals().get("Terrain")?;
        let airdromes: LuaTable<'_> = terrain.call_function("GetTerrainConfig", "Airdromes")?;

        for pair in airdromes.pairs::<usize, LuaTable<'_>>() {
            let (_, airdrome) = pair?;
            let display_name: String = airdrome.get("display_name")?;

            let (x, y) = {
                let reference_point: LuaTable<'_> = airdrome.get("reference_point")?;
                let x: f64 = reference_point.get("x")?;
                let y: f64 = reference_point.get("y")?;
                (x, y)
            };

            let mut runways: Vec<String> = Vec::new();
            let rwys: LuaTable<'_> = airdrome.get("runways")?;
            for pair in rwys.pairs::<usize, LuaTable<'_>>() {
                let (_, rwy) = pair?;
                let start: String = rwy.get("start")?;
                let end: String = rwy.get("end")?;
                runways.push(start);
                runways.push(end);
            }

            airfields.insert(
                display_name.clone(),
                Airfield {
                    name: display_name,
                    position: Position { x, y, alt: 0.0 },
                    runways,
                    traffic_freq: None,
                    info_ltr_offset: rng.gen_range(0..25),
                    info_ltr_override: None,
                    active_rwy_override: None,
                    qnh_override: None,
                },
            );
        }

        airfields
    };

    // extract all mission statics and ship units to later look for ATIS configs in their names
    let mut mission_units = {
        let current_mission: LuaTable<'_> = lua.globals().get("_current_mission")?;
        let mission: LuaTable<'_> = current_mission.get("mission")?;
        let coalitions: LuaTable<'_> = mission.get("coalition")?;

        let mut mission_units = Vec::new();

        for key in &["blue", "red", "neutrals"] {
            let coalition: LuaTable<'_> = coalitions.get(*key)?;
            let countries: LuaTable<'_> = coalition.get("country")?;

            for country in countries.sequence_values::<LuaTable<'_>>() {
                // `_current_mission.mission.coalition.{blue,red,neutrals}.country[i].{static|plane|helicopter|vehicle|ship}.group[j]
                let country = country?;
                let keys = vec!["static", "plane", "helicopter", "vehicle", "ship"];
                for key in keys {
                    if let Some(assets) = country.get::<_, Option<LuaTable<'_>>>(key)? {
                        if let Some(groups) = assets.get::<_, Option<LuaTable<'_>>>("group")? {
                            for group in groups.sequence_values::<LuaTable<'_>>() {
                                let group = group?;
                                if let Some(units) =
                                    group.get::<_, Option<LuaTable<'_>>>("units")?
                                {
                                    for unit in units.sequence_values::<LuaTable<'_>>() {
                                        let unit = unit?;
                                        let x: f64 = unit.get("x")?;
                                        let y: f64 = unit.get("y")?;
                                        let alt: Option<f64> = unit.get("alt").ok();
                                        let unit_id: u32 = unit.get("unitId")?;

                                        mission_units.push(MissionUnit {
                                            id: unit_id,
                                            name: String::new(),
                                            x,
                                            y,
                                            alt: alt.unwrap_or(0.0),
                                            is_static: key == "static",
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        mission_units
    };

    // extract the names for all units
    {
        // read `DCS.getUnitProperty`
        let dcs: LuaTable<'_> = lua.globals().get("DCS")?;
        for mut unit in &mut mission_units {
            // 3 = DCS.UNIT_NAME
            unit.name = dcs.call_function("getUnitProperty", (unit.id, 3))?;
        }
    }

    // read the terrain height for all airdromes and units
    {
        // read `Terrain.GetHeight`
        let terrain: LuaTable<'_> = lua.globals().get("Terrain")?;

        for mut airfield in airfields.values_mut() {
            airfield.position.alt =
                terrain.call_function("GetHeight", (airfield.position.x, airfield.position.y))?;
        }

        for mut unit in &mut mission_units {
            if unit.alt == 0.0 {
                unit.alt = terrain.call_function("GetHeight", (unit.x, unit.y))?;
            }
        }
    }

    // YOLO initialize the atmosphere, because DCS initializes it only after hitting the
    // "Briefing" button, which is something most of the time not done for "dedicated" servers
    {
        lua.load(
            r#"
                local Weather = require 'Weather'
                Weather.initAtmospere(_current_mission.mission.weather)
            "#,
        )
        .exec()?;
    }

    // initialize the dynamic weather component
    let ipc = MissionRpc::new();

    // combine the frequencies that have extracted from the mission's situation with their
    // corresponding airfield
    let mut stations: Vec<Station> = station_configs_from_description
        .into_iter()
        .filter_map(|(name, config)| {
            airfields.remove(&name).map(|mut airfield| {
                airfield.traffic_freq = config.traffic;
                airfield.info_ltr_override = config.info_ltr_override;
                airfield.active_rwy_override = config.active_rwy_override;
                airfield.qnh_override = config.qnh_override;

                Station {
                    name,
                    freq: config.atis,
                    tts: config.tts.unwrap_or_else(|| default_voice.clone()),
                    transmitter: Transmitter::Airfield(airfield),
                    ipc: Some(ipc.clone()),
                }
            })
        })
        .collect();

    // check all units if they represent an ATIS station and if so, combine them with
    // their corresponding airfield
    stations.extend(mission_units.iter().filter_map(|mission_unit| {
        extract_atis_station_config(&mission_unit.name).and_then(|config| {
            airfields.remove(&config.name).map(|mut airfield| {
                airfield.traffic_freq = config.traffic;
                airfield.info_ltr_override = config.info_ltr_override;
                airfield.active_rwy_override = config.active_rwy_override;
                airfield.qnh_override = config.qnh_override;
                airfield.position.x = mission_unit.x;
                airfield.position.y = mission_unit.y;
                airfield.position.alt = mission_unit.alt;

                Station {
                    name: config.name,
                    freq: config.atis,
                    tts: config.tts.unwrap_or_else(|| default_voice.clone()),
                    transmitter: Transmitter::Airfield(airfield),
                    ipc: Some(ipc.clone()),
                }
            })
        })
    }));

    if stations.is_empty() {
        log::info!("No ATIS stations found");
    } else {
        log::info!("ATIS Stations:");
        for station in &stations {
            log::info!(
                "  - {} (Freq: {}, Voice: {:?})",
                station.name,
                station.freq,
                station.tts
            );
        }
    }

    let carriers = mission_units
        .iter()
        .filter_map(|mission_unit| {
            extract_carrier_station_config(&mission_unit.name).map(|config| Station {
                name: config.name.clone(),
                freq: config.atis,
                tts: config.tts.unwrap_or_else(|| default_voice.clone()),
                transmitter: Transmitter::Carrier(Carrier {
                    name: config.name,
                    unit_id: mission_unit.id,
                    unit_name: mission_unit.name.clone(),
                }),
                ipc: Some(ipc.clone()),
            })
        })
        .collect::<Vec<_>>();

    if carriers.is_empty() {
        log::info!("No Carrier stations found");
    } else {
        log::info!("Carrier Stations:");
        for station in &carriers {
            log::info!(
                "  - {} (Freq: {}, Voice: {:?})",
                station.name,
                station.freq,
                station.tts
            );
        }
    }

    let broadcasts = mission_units
        .iter()
        .filter_map(|mission_unit| {
            extract_custom_broadcast_config(&mission_unit.name).map(|config| Station {
                name: mission_unit.name.clone(),
                freq: config.freq,
                tts: config.tts.unwrap_or_else(|| default_voice.clone()),
                transmitter: Transmitter::Custom(Custom {
                    position: if mission_unit.is_static {
                        Some(Position {
                            x: mission_unit.x,
                            y: mission_unit.y,
                            alt: mission_unit.alt,
                        })
                    } else {
                        None
                    },
                    unit_id: mission_unit.id,
                    unit_name: mission_unit.name.clone(),
                    message: config.message,
                }),
                ipc: Some(ipc.clone()),
            })
        })
        .collect::<Vec<_>>();

    if broadcasts.is_empty() {
        log::info!("No custom Broadcast stations found");
    } else {
        log::info!("Broadcast Stations:");
        for station in &broadcasts {
            log::info!(
                "  - {} (Freq: {}, Voice: {:?})",
                station.name,
                station.freq,
                station.tts
            );
        }
    }

    let weather_stations = mission_units
        .iter()
        .filter_map(|mission_unit| {
            extract_weather_station_config(&mission_unit.name).map(|config| Station {
                name: mission_unit.name.clone(),
                freq: config.freq,
                tts: config.tts.unwrap_or_else(|| default_voice.clone()),
                transmitter: Transmitter::Weather(WeatherTransmitter {
                    position: if mission_unit.is_static {
                        Some(Position {
                            x: mission_unit.x,
                            y: mission_unit.y,
                            alt: mission_unit.alt,
                        })
                    } else {
                        None
                    },
                    name: config.name,
                    unit_id: mission_unit.id,
                    unit_name: mission_unit.name.clone(),
                    info_ltr_offset: rng.gen_range(0..25),
                    info_ltr_override: None,
                }),
                ipc: Some(ipc.clone()),
            })
        })
        .collect::<Vec<_>>();

    if weather_stations.is_empty() {
        log::info!("No weather stations found");
    } else {
        log::info!("Weather Stations:");
        for station in &weather_stations {
            log::info!(
                "  - {} (Freq: {}, Voice: {:?})",
                station.name,
                station.freq,
                station.tts
            );
        }
    }

    stations.extend(carriers);
    stations.extend(broadcasts);
    stations.extend(weather_stations);

    Ok(Info { stations, ipc })
}

#[derive(Debug)]
struct MissionUnit {
    id: u32,
    name: String,
    x: f64,
    y: f64,
    alt: f64,
    is_static: bool,
}
