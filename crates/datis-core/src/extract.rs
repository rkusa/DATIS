use std::collections::HashMap;
use std::str::FromStr;

use crate::tts::TextToSpeechProvider;
use regex::{Regex, RegexBuilder};

#[derive(Debug, PartialEq)]
pub struct StationConfig {
    pub name: String,
    pub atis: u64,
    pub traffic: Option<u64>,
    pub tts: Option<TextToSpeechProvider>,
    pub info_ltr_override: Option<char>,
    pub active_rwy_override: Option<String>,
}

pub fn extract_atis_station_frequencies(situation: &str) -> HashMap<String, StationConfig> {
    // extract ATIS stations and frequencies
    let re = Regex::new(r"ATIS ([a-zA-Z- ]+) ([1-3]\d{2}(\.\d{1,3})?)").unwrap();
    let mut stations: HashMap<String, StationConfig> = re
        .captures_iter(situation)
        .map(|caps| {
            let name = caps.get(1).unwrap().as_str().to_string();
            let freq = caps.get(2).unwrap().as_str();
            let freq = (f32::from_str(freq).unwrap() * 1_000_000.0) as u64;
            (
                name.clone(),
                StationConfig {
                    name,
                    atis: freq,
                    traffic: None,
                    tts: None,
                    info_ltr_override: None,
                    active_rwy_override: None,
                },
            )
        })
        .collect();

    // extract optional traffic frequencies
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

pub fn extract_atis_station_config(config: &str) -> Option<StationConfig> {
    let re = RegexBuilder::new(r"^ATIS ([a-zA-Z- ]+) ([1-3]\d{2}(\.\d{1,3})?)")
        .case_insensitive(true)
        .build()
        .unwrap();

    let caps = re.captures(config)?;
    let name = caps.get(1).unwrap().as_str().to_string();
    let atis_freq = caps.get(2).unwrap();
    let atis_freq = (f64::from_str(atis_freq.as_str()).unwrap() * 1_000_000.0) as u64;

    let mut traffic_freq: Option<u64> = None;
    let mut tts: Option<TextToSpeechProvider> = None;
    let mut info_ltr_override = None;
    let mut active_rwy_override = None;

    let rex_option = RegexBuilder::new(r"([^ ]*) (.*)")
        .case_insensitive(true)
        .build()
        .unwrap();
    for token in config.split(",").skip(1) {
        let caps = rex_option.captures(token.trim()).unwrap();
        let option_key = caps.get(1).unwrap().as_str();
        let option_value = caps.get(2).map_or("", |m| m.as_str());

        match option_key {
            "TRAFFIC" => {
                if let Ok(traffic_freq_hz) = option_value.parse::<f64>() {
                    traffic_freq = Some((traffic_freq_hz * 1_000_000.0) as u64);
                } else {
                    log::warn!(
                        "Unable to extract ATIS station traffic frequency from {}",
                        option_value
                    );
                }
            }
            "VOICE" => {
                if let Ok(tts_provider) = TextToSpeechProvider::from_str(option_value) {
                    tts = Some(tts_provider);
                } else {
                    log::warn!("Unable to extract Voice from {}", option_value);
                }
            }
            "INFO" => {
                info_ltr_override = caps.get(2).map_or(None, |param| {
                    Some(param.as_str().chars().next().unwrap().to_ascii_uppercase())
                });
            }
            "ACTIVE" => {
                active_rwy_override = caps
                    .get(2)
                    .map_or(None, |param| Some(param.as_str().into()));
            }
            _ => {
                log::warn!("Unsupported ATIS station option {}", option_key);
            }
        }
    }

    let result = StationConfig {
        name: name,
        atis: atis_freq,
        traffic: traffic_freq,
        tts,
        info_ltr_override,
        active_rwy_override,
    };

    Some(result)
}

pub fn extract_carrier_station_config(config: &str) -> Option<StationConfig> {
    let re = RegexBuilder::new(r"^CARRIER ([a-zA-Z- ]+) ([1-3]\d{2}(\.\d{1,3})?)")
        .case_insensitive(true)
        .build()
        .unwrap();

    let caps = re.captures(config)?;
    let name = caps.get(1).unwrap().as_str().to_string();
    let atis_freq = caps.get(2).unwrap();
    let atis_freq = (f64::from_str(atis_freq.as_str()).unwrap() * 1_000_000.0) as u64;

    let mut tts: Option<TextToSpeechProvider> = None;
    let mut info_ltr_override = None;

    for token in config.split(",").skip(1) {
        let token = token.trim();
        let (option_key, option_value) = token.split_at(token.find(' ').unwrap_or(token.len()));
        let option_key = option_key.trim();
        let option_value = option_value.trim();

        match option_key {
            "VOICE" => {
                if let Ok(tts_provider) = TextToSpeechProvider::from_str(option_value) {
                    tts = Some(tts_provider);
                } else {
                    log::warn!("Unable to extract Voice from {}", option_value);
                }
            }
            "INFO" => {
                info_ltr_override = caps.get(2).map_or(None, |param| {
                    Some(param.as_str().chars().next().unwrap().to_ascii_uppercase())
                });
            }
            _ => {
                log::warn!("Unsupported CARRIER station option {}", option_key);
            }
        }
    }

    let result = StationConfig {
        name: name,
        atis: atis_freq,
        traffic: None,
        tts,
        info_ltr_override,
        active_rwy_override: None,
    };

    Some(result)
}

#[derive(Debug, PartialEq)]
pub struct BroadcastConfig {
    pub freq: u64,
    pub message: String,
    pub tts: Option<TextToSpeechProvider>,
}

pub fn extract_custom_broadcast_config(config: &str) -> Option<BroadcastConfig> {
    let re = RegexBuilder::new(r"^BROADCAST ([1-3]\d{2}(\.\d{1,3})?)(.*): ([^:]+)$")
        .case_insensitive(true)
        .build()
        .unwrap();

    let caps = re.captures(config)?;
    let freq = caps.get(1).unwrap();
    let options = caps.get(3);
    let freq = (f64::from_str(freq.as_str()).unwrap() * 1_000_000.0) as u64;
    let message = caps.get(4).unwrap().as_str().to_string();

    let mut tts: Option<TextToSpeechProvider> = None;
    if options.is_some() {
        for token in options.unwrap().as_str().split(",").skip(1) {
            let token = token.trim();
            let (option_key, option_value) = token.split_at(token.find(' ').unwrap_or(token.len()));
            let option_key = option_key.trim();
            let option_value = option_value.trim();

            match option_key {
                "VOICE" => {
                    if let Ok(tts_provider) = TextToSpeechProvider::from_str(option_value) {
                        tts = Some(tts_provider);
                    } else {
                        log::warn!("Unable to extract Voice from {}", option_value);
                    }
                }
                _ => {
                    log::warn!("Unsupported BROADCAST station option {}", option_key);
                }
            }
        }
    }

    let result = BroadcastConfig { freq, message, tts };

    Some(result)
}

#[derive(Debug, PartialEq)]
pub struct WetherStationConfig {
    pub name: String,
    pub freq: u64,
    pub tts: Option<TextToSpeechProvider>,
}

pub fn extract_weather_station_config(config: &str) -> Option<WetherStationConfig> {
    let re = RegexBuilder::new(r"^WEATHER ([a-zA-Z- ]+) ([1-3]\d{2}(\.\d{1,3})?)")
        .case_insensitive(true)
        .build()
        .unwrap();

    let caps = re.captures(config)?;
    let name = caps.get(1).unwrap().as_str().to_string();
    let station_freq = caps.get(2).unwrap();
    let station_freq = (f64::from_str(station_freq.as_str()).unwrap() * 1_000_000.0) as u64;

    let mut tts: Option<TextToSpeechProvider> = None;

    for token in config.split(",").skip(1) {
        let token = token.trim();
        let (option_key, option_value) = token.split_at(token.find(' ').unwrap_or(token.len()));
        let option_key = option_key.trim();
        let option_value = option_value.trim();

        match option_key {
            "VOICE" => {
                if let Ok(tts_provider) = TextToSpeechProvider::from_str(option_value) {
                    tts = Some(tts_provider);
                } else {
                    log::warn!("Unable to extract Voice from {}", option_value);
                }
            }
            _ => {
                log::warn!("Unsupported WEATHER station option {}", option_key);
            }
        }
    }

    let result = WetherStationConfig {
        name: name,
        freq: station_freq,
        tts,
    };

    Some(result)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tts::{aws, gcloud, TextToSpeechProvider};

    #[test]
    fn test_mission_situation_extraction() {
        let freqs = extract_atis_station_frequencies(
            r#"
            ATIS Mineralnye Vody 251.000
            ATIS Batumi 131.5
            ATIS Senaki-Kolkhi 145

            TRAFFIC Batumi 255.00
        "#,
        );

        assert_eq!(
            freqs,
            vec![
                (
                    "Mineralnye Vody".to_string(),
                    StationConfig {
                        name: "Mineralnye Vody".to_string(),
                        atis: 251_000_000,
                        traffic: None,
                        tts: None,
                        info_ltr_override: None,
                        active_rwy_override: None,
                    }
                ),
                (
                    "Batumi".to_string(),
                    StationConfig {
                        name: "Batumi".to_string(),
                        atis: 131_500_000,
                        traffic: Some(255_000_000),
                        tts: None,
                        info_ltr_override: None,
                        active_rwy_override: None,
                    }
                ),
                (
                    "Senaki-Kolkhi".to_string(),
                    StationConfig {
                        name: "Senaki-Kolkhi".to_string(),
                        atis: 145_000_000,
                        traffic: None,
                        tts: None,
                        info_ltr_override: None,
                        active_rwy_override: None,
                    }
                )
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn test_atis_config_extraction() {
        assert_eq!(
            extract_atis_station_config("ATIS Kutaisi 251"),
            Some(StationConfig {
                name: "Kutaisi".to_string(),
                atis: 251_000_000,
                traffic: None,
                tts: None,
                info_ltr_override: None,
                active_rwy_override: None,
            })
        );

        assert_eq!(
            extract_atis_station_config("ATIS Mineralnye Vody 251"),
            Some(StationConfig {
                name: "Mineralnye Vody".to_string(),
                atis: 251_000_000,
                traffic: None,
                tts: None,
                info_ltr_override: None,
                active_rwy_override: None,
            })
        );

        assert_eq!(
            extract_atis_station_config("ATIS Senaki-Kolkhi 251"),
            Some(StationConfig {
                name: "Senaki-Kolkhi".to_string(),
                atis: 251_000_000,
                traffic: None,
                tts: None,
                info_ltr_override: None,
                active_rwy_override: None,
            })
        );

        assert_eq!(
            extract_atis_station_config("ATIS Kutaisi 251.000, TRAFFIC 123.45"),
            Some(StationConfig {
                name: "Kutaisi".to_string(),
                atis: 251_000_000,
                traffic: Some(123_450_000),
                tts: None,
                info_ltr_override: None,
                active_rwy_override: None,
            })
        );

        assert_eq!(
            extract_atis_station_config(
                "ATIS Kutaisi 251.000, TRAFFIC 123.45, VOICE en-US-Standard-E, INFO Q"
            ),
            Some(StationConfig {
                name: "Kutaisi".to_string(),
                atis: 251_000_000,
                traffic: Some(123_450_000),
                tts: Some(TextToSpeechProvider::GoogleCloud {
                    voice: gcloud::VoiceKind::StandardE
                }),
                info_ltr_override: Some('Q'),
                active_rwy_override: None,
            })
        );

        assert_eq!(
            extract_atis_station_config(
                "ATIS Kutaisi 251.000, TRAFFIC 123.45, VOICE en-US-Standard-E"
            ),
            Some(StationConfig {
                name: "Kutaisi".to_string(),
                atis: 251_000_000,
                traffic: Some(123_450_000),
                tts: Some(TextToSpeechProvider::GoogleCloud {
                    voice: gcloud::VoiceKind::StandardE
                }),
                info_ltr_override: None,
                active_rwy_override: None,
            })
        );

        assert_eq!(
            extract_atis_station_config("ATIS Kutaisi 251.000, VOICE en-US-Standard-E"),
            Some(StationConfig {
                name: "Kutaisi".to_string(),
                atis: 251_000_000,
                traffic: None,
                tts: Some(TextToSpeechProvider::GoogleCloud {
                    voice: gcloud::VoiceKind::StandardE
                }),
                info_ltr_override: None,
                active_rwy_override: None,
            })
        );

        assert_eq!(
            extract_atis_station_config("ATIS Kutaisi 131.400"),
            Some(StationConfig {
                name: "Kutaisi".to_string(),
                atis: 131_400_000,
                traffic: None,
                tts: None,
                info_ltr_override: None,
                active_rwy_override: None,
            })
        );

        // Test handling invalid value
        assert_eq!(
            extract_atis_station_config("ATIS Kutaisi 131.400, TRAFFIC Potatoe"),
            Some(StationConfig {
                name: "Kutaisi".to_string(),
                atis: 131_400_000,
                traffic: None,
                tts: None,
                info_ltr_override: None,
                active_rwy_override: None,
            })
        );

        // Test handling invalid key
        assert_eq!(
            extract_atis_station_config("ATIS Kutaisi 131.400, GRAVITY 7"),
            Some(StationConfig {
                name: "Kutaisi".to_string(),
                atis: 131_400_000,
                traffic: None,
                tts: None,
                info_ltr_override: None,
                active_rwy_override: None,
            })
        );
    }

    #[test]
    fn test_carrier_config_extraction() {
        assert_eq!(
            extract_carrier_station_config("CARRIER Mother 251"),
            Some(StationConfig {
                name: "Mother".to_string(),
                atis: 251_000_000,
                traffic: None,
                tts: None,
                info_ltr_override: None,
                active_rwy_override: None,
            })
        );

        assert_eq!(
            extract_carrier_station_config("CARRIER Mother 131.400"),
            Some(StationConfig {
                name: "Mother".to_string(),
                atis: 131_400_000,
                traffic: None,
                tts: None,
                info_ltr_override: None,
                active_rwy_override: None,
            })
        );

        assert_eq!(
            extract_carrier_station_config("CARRIER Mother 251.000, VOICE en-US-Standard-E"),
            Some(StationConfig {
                name: "Mother".to_string(),
                atis: 251_000_000,
                traffic: None,
                tts: Some(TextToSpeechProvider::GoogleCloud {
                    voice: gcloud::VoiceKind::StandardE
                }),
                info_ltr_override: None,
                active_rwy_override: None,
            })
        );
    }

    #[test]
    fn test_cloud_provider_prefix_extraction() {
        assert_eq!(
            extract_atis_station_config("ATIS Kutaisi 131.400, VOICE GC:en-US-Standard-D"),
            Some(StationConfig {
                name: "Kutaisi".to_string(),
                atis: 131_400_000,
                traffic: None,
                tts: Some(TextToSpeechProvider::GoogleCloud {
                    voice: gcloud::VoiceKind::StandardD
                }),
                info_ltr_override: None,
                active_rwy_override: None,
            })
        );

        assert_eq!(
            extract_atis_station_config("ATIS Kutaisi 131.400, VOICE AWS:Brian"),
            Some(StationConfig {
                name: "Kutaisi".to_string(),
                atis: 131_400_000,
                traffic: None,
                tts: Some(TextToSpeechProvider::AmazonWebServices {
                    voice: aws::VoiceKind::Brian
                }),
                info_ltr_override: None,
                active_rwy_override: None,
            })
        );
    }

    #[test]
    fn test_reordered_parameters() {
        // Test parameters in another order
        assert_eq!(
            extract_atis_station_config(
                "ATIS Kutaisi 251.000, VOICE en-US-Standard-E, TRAFFIC 123.45"
            ),
            Some(StationConfig {
                name: "Kutaisi".to_string(),
                atis: 251_000_000,
                traffic: Some(123_450_000),
                tts: Some(TextToSpeechProvider::GoogleCloud {
                    voice: gcloud::VoiceKind::StandardE
                }),
                info_ltr_override: None,
                active_rwy_override: None,
            })
        );
    }

    #[test]
    fn test_complete_garbage() {
        assert_eq!(
            extract_atis_station_config("not an atis station at all"),
            None
        );

        assert_eq!(
            extract_carrier_station_config("not a carrer station at all"),
            None
        );

        assert_eq!(
            extract_custom_broadcast_config("not a custom broadcast at all"),
            None
        );

        assert_eq!(
            extract_weather_station_config("not a weather station at all"),
            None
        );
    }

    #[test]
    fn test_active_rwy_override() {
        assert_eq!(
            extract_atis_station_config("ATIS Kutaisi 131.400, ACTIVE 21L"),
            Some(StationConfig {
                name: "Kutaisi".to_string(),
                atis: 131_400_000,
                traffic: None,
                tts: None,
                info_ltr_override: None,
                active_rwy_override: Some("21L".to_string()),
            })
        );
    }

    #[test]
    fn test_broadcast_config_extraction() {
        assert_eq!(
            extract_custom_broadcast_config("BROADCAST 251: Bla bla"),
            Some(BroadcastConfig {
                freq: 251_000_000,
                message: "Bla bla".to_string(),
                tts: None,
            })
        );

        assert_eq!(
            extract_custom_broadcast_config("BROADCAST 251.500, VOICE AWS:Brian: Bla bla"),
            Some(BroadcastConfig {
                freq: 251_500_000,
                message: "Bla bla".to_string(),
                tts: Some(TextToSpeechProvider::AmazonWebServices {
                    voice: aws::VoiceKind::Brian
                }),
            })
        );
    }

    #[test]
    fn test_weather_station_config_extraction() {
        assert_eq!(
            extract_weather_station_config("WEATHER Shooting Range 251"),
            Some(WetherStationConfig {
                name: "Shooting Range".to_string(),
                freq: 251_000_000,
                tts: None,
            })
        );

        assert_eq!(
            extract_weather_station_config("WEATHER Coast 131.400"),
            Some(WetherStationConfig {
                name: "Coast".to_string(),
                freq: 131_400_000,
                tts: None,
            })
        );

        assert_eq!(
            extract_weather_station_config(
                "WEATHER Mountain Range 251.000, VOICE en-US-Standard-E"
            ),
            Some(WetherStationConfig {
                name: "Mountain Range".to_string(),
                freq: 251_000_000,
                tts: Some(TextToSpeechProvider::GoogleCloud {
                    voice: gcloud::VoiceKind::StandardE
                }),
            })
        );
    }
}
