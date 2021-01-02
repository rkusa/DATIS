use std::collections::HashMap;
use std::str::FromStr;

use datis_core::extract::*;
use datis_core::rpc::*;
use datis_core::station::*;
use datis_core::tts::TextToSpeechProvider;
use datis_core::weather::Clouds;
use mlua::prelude::{Lua, LuaTable, LuaTableExt};
use rand::Rng;

pub struct Info {
    pub stations: Vec<Station>,
    pub gcloud_key: String,
    pub aws_key: String,
    pub aws_secret: String,
    pub aws_region: String,
    pub srs_port: u16,
    pub rpc: MissionRpc,
}

pub fn extract(lua: &Lua) -> Result<Info, mlua::Error> {
    let options = get_options(lua)?;
    log::info!("Using SRS Server port: {}", options.srs_port);

    // extract frequencies from mission briefing, which is retrieved from
    // `DCS.getMissionDescription()`
    let station_configs_from_description = {
        let dcs: LuaTable<'_> = lua.globals().get("DCS")?;
        let mission_description: String = dcs.call_function("getMissionDescription", ())?;
        extract_stationc_config_from_mission_description(&mission_description)
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
                    info_ltr_offset: rng.gen_range(0, 25),
                    info_ltr_override: None,
                    active_rwy_override: None,
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

    // extract the current mission's weather kind and static weather configuration
    let (clouds, fog_thickness, fog_visibility) = {
        // read `_current_mission.mission.weather`
        let current_mission: LuaTable<'_> = lua.globals().get("_current_mission")?;
        let mission: LuaTable<'_> = current_mission.get("mission")?;
        let weather: LuaTable<'_> = mission.get("weather")?;

        // read `_current_mission.mission.weather.atmosphere_type`
        let atmosphere_type: f64 = weather.get("atmosphere_type")?;
        let is_dynamic = atmosphere_type != 0.0;

        let clouds = {
            if is_dynamic {
                None
            } else {
                let clouds: LuaTable<'_> = weather.get("clouds")?;
                Some(Clouds {
                    base: clouds.get("base")?,
                    density: clouds.get("density")?,
                    thickness: clouds.get("thickness")?,
                    iprecptns: clouds.get("iprecptns")?,
                })
            }
        };

        // Note: `weather.visibility` is always the same, which is why we cannot use it here
        // and use the fog instead to derive some kind of visibility

        let fog: LuaTable<'_> = weather.get("fog")?;
        let fog_thickness: u32 = fog.get("thickness")?;
        let fog_visibility: u32 = fog.get("visibility")?;

        (clouds, fog_thickness, fog_visibility)
    };

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
    let rpc = MissionRpc::new(clouds, fog_thickness, fog_visibility);

    let default_voice = match TextToSpeechProvider::from_str(&options.default_voice) {
        Ok(default_voice) => default_voice,
        Err(err) => {
            log::warn!("Invalid default voice `{}`: {}", options.default_voice, err);
            TextToSpeechProvider::default()
        }
    };

    // combine the frequencies that have extracted from the mission's situation with their
    // corresponding airfield
    let mut stations: Vec<Station> = station_configs_from_description
        .into_iter()
        .filter_map(|(name, config)| {
            airfields.remove(&name).map(|mut airfield| {
                airfield.traffic_freq = config.traffic;
                airfield.info_ltr_override = config.info_ltr_override;
                airfield.active_rwy_override = config.active_rwy_override;

                Station {
                    name,
                    freq: config.atis,
                    tts: default_voice.clone(),
                    transmitter: Transmitter::Airfield(airfield),
                    rpc: Some(rpc.clone()),
                }
            })
        })
        .collect();

    // check all units if they represent and ATIS station and if so, combine them with
    // their corresponding airfield
    stations.extend(mission_units.iter().filter_map(|mission_unit| {
        extract_atis_station_config(&mission_unit.name).and_then(|config| {
            airfields.remove(&config.name).map(|mut airfield| {
                airfield.traffic_freq = config.traffic;
                airfield.info_ltr_override = config.info_ltr_override;
                airfield.active_rwy_override = config.active_rwy_override;
                airfield.position.x = mission_unit.x;
                airfield.position.y = mission_unit.y;
                airfield.position.alt = mission_unit.alt;

                Station {
                    name: config.name,
                    freq: config.atis,
                    tts: config.tts.unwrap_or_else(|| default_voice.clone()),
                    transmitter: Transmitter::Airfield(airfield),
                    rpc: Some(rpc.clone()),
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
                rpc: Some(rpc.clone()),
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
                rpc: Some(rpc.clone()),
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
                    info_ltr_offset: rng.gen_range(0, 25),
                    info_ltr_override: None,
                }),
                rpc: Some(rpc.clone()),
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

    Ok(Info {
        stations,
        gcloud_key: options.gcloud_key,
        aws_key: options.aws_key,
        aws_secret: options.aws_secret,
        aws_region: options.aws_region,
        srs_port: options.srs_port,
        rpc,
    })
}

struct Options {
    default_voice: String,
    gcloud_key: String,
    aws_key: String,
    aws_secret: String,
    aws_region: String,
    srs_port: u16,
}

fn get_options(lua: &Lua) -> Result<Options, mlua::Error> {
    let options_data: LuaTable<'_> = lua.globals().get("OptionsData")?;

    // OptionsData.getPlugin("DATIS", "defaultVoice")
    let default_voice = options_data.call_function("getPlugin", ("DATIS", "defaultVoice"))?;
    let gcloud_key = options_data.call_function("getPlugin", ("DATIS", "gcloudAccessKey"))?;
    let aws_key = options_data.call_function("getPlugin", ("DATIS", "awsAccessKey"))?;
    let aws_secret = options_data.call_function("getPlugin", ("DATIS", "awsPrivateKey"))?;
    let aws_region = options_data.call_function("getPlugin", ("DATIS", "awsRegion"))?;
    let srs_port: u16 = options_data.call_function("getPlugin", ("DATIS", "srsPort"))?;

    Ok(Options {
        default_voice,
        gcloud_key,
        aws_key,
        aws_secret,
        aws_region,
        srs_port,
    })
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
