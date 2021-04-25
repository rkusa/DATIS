use crate::station::Position;
use crate::utils::m_to_ft;
use serde::Deserialize;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct WeatherInfo {
    pub clouds: Option<Clouds>,
    pub wind_speed: f64,     // in m/s
    pub wind_dir: f64,       // in degrees (the direction the wind is coming from)
    pub temperature: f64,    // in °C
    pub pressure_qnh: f64,   // in N/m2
    pub pressure_qfe: f64,   // in N/m2
    pub fog_thickness: f64,  // in m
    pub fog_visibility: f64, // in m
    pub dust_density: u32,
    pub position: Position,
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Clouds {
    New(NewClouds),
    Old(OldClouds),
}

#[derive(Debug, PartialEq, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewClouds {
    pub base: u32, // in m
    pub preset: CloudPreset,
}

#[derive(Debug, PartialEq, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudPreset {
    pub precipitation_power: f64,
    pub layers: Vec<NewCloudLayer>,
}

#[derive(Debug, PartialEq, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewCloudLayer {
    altitude_min: u32, // in m
    altitude_max: u32, // in m
    coverage: f64,
}

#[derive(Debug, PartialEq, Clone, Default, Deserialize)]
pub struct OldClouds {
    pub base: u32, // in m
    pub density: u32,
    pub thickness: u32,
    pub iprecptns: u32,
}

impl WeatherInfo {
    /// Get QNH correct for the current temperature (as far as possible in DCS)
    pub fn get_qnh(&self, alt: u32) -> f64 {
        self.pressure_qnh + self.pressure_correction(alt)
    }

    /// Get QFE correct for the current temperature (as far as possible in DCS)
    pub fn get_qfe(&self, alt: u32) -> f64 {
        self.pressure_qfe + self.pressure_correction(alt)
    }

    fn pressure_correction(&self, alt: u32) -> f64 {
        let alt = m_to_ft(alt as f64);
        let angels = alt / 1000.0;
        // ISA at see level is 16 and not 15 in DCS, see
        // https://forums.eagle.ru/topic/256057-altitude-qnh-error-bug/
        let isa_at_alt = 16.0 - 1.98 * angels;
        let isa_diff = self.temperature - isa_at_alt;
        let palt_diff = 4.0 * isa_diff * angels;

        // translate alt diff into a QNH diff
        let qnh_diff = (palt_diff / 27.0) * 100.0;
        qnh_diff
    }

    /// in m
    pub fn get_visibility(&self, alt: u32) -> Option<u32> {
        let clouds_vis = self.clouds.as_ref().and_then(|c| c.get_visibility(alt));
        let dust_vis = self.get_dust_storm_visibility(alt);
        let fog_vis = self.get_fog_visibility(alt);
        let vis = vec![clouds_vis, dust_vis, fog_vis]
            .into_iter()
            .filter_map(|e| e)
            .min();
        // Visibility below 50m is considered as ZERO
        vis.map(|v| if v < 50 { 0 } else { v })
    }

    /// in m
    fn get_dust_storm_visibility(&self, alt: u32) -> Option<u32> {
        if self.dust_density == 0 || alt >= 50 {
            return None;
        }

        // The multiplier of 4 was derived by manual testing the resulting visibility.
        Some(self.dust_density * 4)
    }

    /// in m
    fn get_fog_visibility(&self, alt: u32) -> Option<u32> {
        if self.fog_visibility == 0.0 || alt as f64 > self.fog_thickness {
            return None;
        }

        Some(self.fog_visibility.round() as u32)
    }

    /// in ft
    pub fn get_ceiling(&self, alt: u32) -> Option<Ceiling> {
        for layer in self.get_cloud_layers() {
            if (layer.altitude_min..=layer.altitude_max).contains(&alt) {
                return None;
            }

            if layer.altitude_min > alt
                && matches!(
                    layer.coverage,
                    CloudCoverage::Broken | CloudCoverage::Overcast
                )
            {
                return Some(Ceiling {
                    alt: m_to_ft(layer.altitude_min as f64),
                    coverage: layer.coverage,
                });
            }
        }

        None
    }

    pub fn get_cloud_layers(&self) -> Vec<CloudLayer> {
        self.clouds
            .as_ref()
            .map(|c| c.get_cloud_layers())
            .unwrap_or_default()
    }

    pub fn get_weather_conditions(&self, alt: u32) -> Vec<WeatherCondition> {
        let mut kind = self
            .clouds
            .as_ref()
            .map(|c| c.get_weather_conditions())
            .unwrap_or_default();
        if self.get_dust_storm_visibility(alt).is_some() {
            kind.push(WeatherCondition::DustStorm);
        }
        if self.get_fog_visibility(alt).is_some() {
            kind.push(WeatherCondition::Fog);
        }
        kind
    }
}

pub struct Ceiling {
    /// in ft
    pub alt: f64,
    pub coverage: CloudCoverage,
}

#[derive(Debug, Clone, Copy)]
pub enum CloudCoverage {
    Clear,
    Few,
    Scattered,
    Broken,
    Overcast,
}

#[derive(Debug, Clone, Copy)]
pub enum WeatherCondition {
    SlightRain,
    Rain,
    HeavyRain,
    Thunderstorm,
    Fog,
    DustStorm,
}

pub struct CloudLayer {
    coverage: CloudCoverage,
    altitude_min: u32,
    altitude_max: u32,
}

impl Clouds {
    pub fn get_cloud_layers(&self) -> Vec<CloudLayer> {
        match self {
            Clouds::New(clouds) => clouds.get_cloud_layers(),
            Clouds::Old(clouds) => clouds.get_cloud_layers(),
        }
    }

    pub fn get_weather_conditions(&self) -> Vec<WeatherCondition> {
        match self {
            Clouds::New(clouds) => clouds.get_weather_conditions(),
            Clouds::Old(clouds) => clouds.get_weather_conditions(),
        }
    }

    /// in meters
    pub fn get_visibility(&self, alt: u32) -> Option<u32> {
        for layer in self.get_cloud_layers() {
            if alt >= layer.altitude_min
                && alt <= layer.altitude_max
                && matches!(
                    layer.coverage,
                    CloudCoverage::Scattered | CloudCoverage::Broken | CloudCoverage::Overcast
                )
            {
                return Some(0);
            }
        }

        match self {
            Clouds::New(clouds) => clouds.get_visibility(),
            Clouds::Old(clouds) => clouds.get_visibility(),
        }
    }
}

impl NewClouds {
    pub fn get_cloud_layers(&self) -> Vec<CloudLayer> {
        let diff = match self.preset.layers.first() {
            Some(first) => self.base - first.altitude_min,
            None => return Vec::new(),
        };

        self.preset
            .layers
            .iter()
            .map(|layer| CloudLayer {
                coverage: match layer.coverage {
                    x if (0.0..0.3).contains(&x) => CloudCoverage::Clear,
                    x if (0.3..0.5).contains(&x) => CloudCoverage::Few,
                    x if (0.5..0.6).contains(&x) => CloudCoverage::Scattered,
                    x if (0.6..0.9).contains(&x) => CloudCoverage::Broken,
                    x if (0.9..f64::MAX).contains(&x) => CloudCoverage::Overcast,
                    _ => CloudCoverage::Clear, // unreachable
                },
                altitude_min: layer.altitude_min + diff,
                altitude_max: layer.altitude_max + diff,
            })
            .collect()
    }

    pub fn get_weather_conditions(&self) -> Vec<WeatherCondition> {
        if self.preset.precipitation_power <= 0.0 {
            return vec![];
        }

        match self.preset.precipitation_power {
            x if (0.0..0.5).contains(&x) => vec![WeatherCondition::SlightRain],
            x if (0.5..0.8).contains(&x) => vec![WeatherCondition::Rain],
            x if (0.8..f64::MAX).contains(&x) => vec![WeatherCondition::HeavyRain],
            _ => vec![], // unreachable
        }
    }

    pub fn get_visibility(&self) -> Option<u32> {
        if self.preset.precipitation_power <= 0.0 {
            return None;
        }

        match self.preset.precipitation_power {
            x if (0.0..0.3).contains(&x) => Some(5_000),
            x if (0.3..0.5).contains(&x) => Some(4_000),
            x if (0.5..0.6).contains(&x) => Some(3_000),
            x if (0.6..0.7).contains(&x) => Some(2_500),
            x if (0.7..0.8).contains(&x) => Some(2_000),
            x if (0.8..0.9).contains(&x) => Some(1_500),
            x if (0.9..0.97).contains(&x) => Some(1_000),
            x if (1.0..f64::MAX).contains(&x) => Some(700),
            _ => None, // unreachable
        }
    }
}

impl OldClouds {
    pub fn get_cloud_layers(&self) -> Vec<CloudLayer> {
        vec![CloudLayer {
            coverage: match self.density {
                x if (0..2).contains(&x) => CloudCoverage::Clear,
                x if (2..4).contains(&x) => CloudCoverage::Few,
                x if (4..6).contains(&x) => CloudCoverage::Scattered,
                x if (6..9).contains(&x) => CloudCoverage::Broken,
                x if (9..u32::MAX).contains(&x) => CloudCoverage::Overcast,
                _ => CloudCoverage::Clear, // unreachable
            },
            altitude_min: if self.base < 1_000 {
                0
            } else {
                self.base - 200
            },
            altitude_max: self.base + self.thickness - 200,
        }]
    }

    pub fn get_weather_conditions(&self) -> Vec<WeatherCondition> {
        match self.iprecptns {
            1 => vec![WeatherCondition::Rain],
            2 => vec![WeatherCondition::Rain, WeatherCondition::Thunderstorm],
            _ => vec![],
        }
    }

    pub fn get_visibility(&self) -> Option<u32> {
        match self.iprecptns {
            1 => Some(7_400),
            2 => Some(1_200),
            _ => None,
        }
    }
}

impl CloudCoverage {
    pub fn to_metar(&self) -> &str {
        match self {
            CloudCoverage::Clear => "CLR",
            CloudCoverage::Few => "FEW",
            CloudCoverage::Scattered => "SCT",
            CloudCoverage::Broken => "BKN",
            CloudCoverage::Overcast => "OVC",
        }
    }
}

impl std::fmt::Display for CloudCoverage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            CloudCoverage::Clear => "Clear",
            CloudCoverage::Few => "Few",
            CloudCoverage::Scattered => "Scattered",
            CloudCoverage::Broken => "Broken",
            CloudCoverage::Overcast => "Overcast",
        })
    }
}

impl WeatherCondition {
    pub fn to_metar(&self) -> &str {
        match self {
            WeatherCondition::SlightRain => "-RA",
            WeatherCondition::Rain => "RA",
            WeatherCondition::HeavyRain => "+RA",
            WeatherCondition::Thunderstorm => "TS",
            WeatherCondition::Fog => "FG",
            WeatherCondition::DustStorm => "DS",
        }
    }
}

impl std::fmt::Display for WeatherCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            WeatherCondition::SlightRain => "Slight Rain",
            WeatherCondition::Rain => "Rain",
            WeatherCondition::HeavyRain => "Heavy Rain",
            WeatherCondition::Thunderstorm => "Thunderstorm",
            WeatherCondition::Fog => "Fog",
            WeatherCondition::DustStorm => "Dust Storm",
        })
    }
}

impl Default for Clouds {
    fn default() -> Self {
        Clouds::Old(OldClouds::default())
    }
}
