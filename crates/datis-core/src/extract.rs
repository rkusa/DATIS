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
    let re = RegexBuilder::new(
        r"^ATIS ([a-zA-Z- ]+) ([1-3]\d{2}(\.\d{1,3})?)(,[ ]?TRAFFIC ([1-3]\d{2}(\.\d{1,3})?))?(,[ ]?VOICE ([a-zA-Z-:]+))?(,[ ]?INFO ([a-zA-Z]))?$",
    )
    .case_insensitive(true)
    .build()
    .unwrap();

    re.captures(config).map(|caps| {
        let name = caps.get(1).unwrap().as_str();
        let atis_freq = caps.get(2).unwrap().as_str();
        let atis_freq = (f64::from_str(atis_freq).unwrap() * 1_000_000.0) as u64;
        let traffic_freq = caps
            .get(5)
            .map(|freq| (f64::from_str(freq.as_str()).unwrap() * 1_000_000.0) as u64);
        let tts = caps
            .get(8)
            .and_then(|s| TextToSpeechProvider::from_str(s.as_str()).ok());
        let info_ltr = caps
            .get(10)
            .map(|ilo| ((ilo.as_str()).chars().next().unwrap().to_ascii_uppercase()) as char);
        StationConfig {
            name: name.to_string(),
            atis: atis_freq,
            traffic: traffic_freq,
            tts,
            info_ltr_override: info_ltr,
        }
    })
}

pub fn extract_carrier_station_config(config: &str) -> Option<StationConfig> {
    let re = RegexBuilder::new(
        r"^CARRIER ([a-zA-Z- ]+) ([1-3]\d{2}(\.\d{1,3})?)(,[ ]?VOICE ([a-zA-Z-:]+))?$",
    )
    .case_insensitive(true)
    .build()
    .unwrap();
    re.captures(config).map(|caps| {
        let name = caps.get(1).unwrap().as_str();
        let atis_freq = caps.get(2).unwrap().as_str();
        let atis_freq = (f64::from_str(atis_freq).unwrap() * 1_000_000.0) as u64;
        let tts = caps
            .get(5)
            .and_then(|s| TextToSpeechProvider::from_str(s.as_str()).ok());
        StationConfig {
            name: name.to_string(),
            atis: atis_freq,
            traffic: None,
            tts,
            info_ltr_override: None
        }
    })
}

#[derive(Debug, PartialEq)]
pub struct BroadcastConfig {
    pub freq: u64,
    pub message: String,
    pub tts: Option<TextToSpeechProvider>,
}

pub fn extract_custom_broadcast_config(config: &str) -> Option<BroadcastConfig> {
    let re = RegexBuilder::new(
        r"^BROADCAST ([1-3]\d{2}(\.\d{1,3})?)(,[ ]?VOICE ([a-zA-Z-:]+))?:[ ]*(.+)$",
    )
    .case_insensitive(true)
    .build()
    .unwrap();
    re.captures(config).map(|caps| {
        let freq = caps.get(1).unwrap().as_str();
        let freq = (f32::from_str(freq).unwrap() * 1_000_000.0) as u64;
        let tts = caps
            .get(4)
            .and_then(|s| TextToSpeechProvider::from_str(s.as_str()).ok());
        let message = caps.get(5).unwrap().as_str();
        BroadcastConfig {
            freq,
            message: message.to_string(),
            tts,
        }
    })
}

#[derive(Debug, PartialEq)]
pub struct WetherStationConfig {
    pub name: String,
    pub freq: u64,
    pub tts: Option<TextToSpeechProvider>,
}

pub fn extract_weather_station_config(config: &str) -> Option<WetherStationConfig> {
    let re = RegexBuilder::new(
        r"^WEATHER ([a-zA-Z- ]+) ([1-3]\d{2}(\.\d{1,3})?)(,[ ]?VOICE ([a-zA-Z-:]+))?$",
    )
    .case_insensitive(true)
    .build()
    .unwrap();
    re.captures(config).map(|caps| {
        let name = caps.get(1).unwrap().as_str();
        let freq = caps.get(2).unwrap().as_str();
        let freq = (f64::from_str(freq).unwrap() * 1_000_000.0) as u64;
        let tts = caps
            .get(5)
            .and_then(|s| TextToSpeechProvider::from_str(s.as_str()).ok());
        WetherStationConfig {
            name: name.to_string(),
            freq,
            tts,
        }
    })
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
            })
        );

        assert_eq!(
            extract_atis_station_config("ATIS Kutaisi 251.000, TRAFFIC 123.45, VOICE en-US-Standard-E, INFO Q"),
            Some(StationConfig {
                name: "Kutaisi".to_string(),
                atis: 251_000_000,
                traffic: Some(123_450_000),
                tts: Some(TextToSpeechProvider::GoogleCloud {
                    voice: gcloud::VoiceKind::StandardE
                }),
                info_ltr_override: Some('Q')
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
            extract_custom_broadcast_config("BROADCAST 251.000, VOICE AWS:Brian: Bla bla"),
            Some(BroadcastConfig {
                freq: 251_000_000,
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
