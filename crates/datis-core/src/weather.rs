use crate::station::Position;
use serde::Deserialize;
use uom::num::Zero;
use uom::num_traits::Pow;
use uom::si::f64::{Angle, Pressure, ThermodynamicTemperature as Temperature, Velocity};
use uom::si::i32::Length;
use uom::si::length::{foot, meter};
use uom::si::pressure::{inch_of_mercury, millibar, pascal};

#[derive(Debug, PartialEq, Clone, Default)]
pub struct WeatherInfo {
    pub clouds: Option<Clouds>,
    pub wind_speed: Velocity,
    /// The direction the wind is coming from
    pub wind_dir: Angle,
    pub temperature: Temperature,
    pub pressure_sealevel: Pressure,
    pub pressure_groundlevel: Pressure,
    /// This basically determines how heigh the fog is, from 0 to `fog_thickness`.
    pub fog_thickness: Length,
    pub fog_visibility: Length,
    pub dust_density: i32,
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
    #[serde(deserialize_with = "crate::de::from_meter")]
    pub base: Length,
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
    #[serde(deserialize_with = "crate::de::from_meter")]
    altitude_min: Length,
    #[serde(deserialize_with = "crate::de::from_meter")]
    altitude_max: Length,
    coverage: f32,
}

#[derive(Debug, PartialEq, Clone, Default, Deserialize)]
pub struct OldClouds {
    #[serde(deserialize_with = "crate::de::from_meter")]
    pub base: Length,
    pub density: u32,
    #[serde(deserialize_with = "crate::de::from_meter")]
    pub thickness: Length,
    pub iprecptns: u32,
}

impl WeatherInfo {
    /// Get QNH correct for the current temperature (as far as possible in DCS)
    pub fn get_qnh(&self, alt: Length) -> Pressure {
        // see https://en.wikipedia.org/wiki/Pressure_altitude
        let pressure_altitude: f64 = 14_5366.45
            * (1.0
                - (self.get_qfe().get::<millibar>() / self.pressure_sealevel.get::<millibar>())
                    .pow(0.190284));
        let alt_diff = f64::from(alt.get::<foot>()) - pressure_altitude;

        // this corrects the pressure for the current temperature
        let correction = Pressure::new::<pascal>(alt_diff / 27.0 * 100.0);

        self.pressure_sealevel + correction
    }

    /// Get QFE
    pub fn get_qfe(&self) -> Pressure {
        self.pressure_groundlevel
    }

    pub fn get_visibility(&self, alt: Length) -> Option<Length> {
        let clouds_vis = self.clouds.as_ref().and_then(|c| c.get_visibility(alt));
        let dust_vis = self.get_dust_storm_visibility(alt);
        let fog_vis = self.get_fog_visibility(alt);
        let vis = vec![clouds_vis, dust_vis, fog_vis]
            .into_iter()
            .filter_map(|e| e)
            .min();
        // Visibility below 50m is considered as ZERO
        vis.map(|v| {
            if v < Length::new::<meter>(50) {
                Length::zero()
            } else {
                v
            }
        })
    }

    fn get_dust_storm_visibility(&self, alt: Length) -> Option<Length> {
        // The dust will only be between 0 and 50 meters altitude
        if self.dust_density == 0 || alt > Length::new::<meter>(50) {
            return None;
        }

        // The multiplier of 4 was derived by manual testing the resulting visibility.
        Some(Length::new::<meter>(self.dust_density * 4))
    }

    fn get_fog_visibility(&self, alt: Length) -> Option<Length> {
        if self.fog_visibility.is_zero() || alt > self.fog_thickness {
            return None;
        }

        Some(self.fog_visibility)
    }

    pub fn get_ceiling(&self, alt: Length) -> Option<Ceiling> {
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
                    alt: layer.altitude_min,
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

    pub fn get_weather_conditions(&self, alt: Length) -> Vec<WeatherCondition> {
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
    pub alt: Length,
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
    altitude_min: Length,
    altitude_max: Length,
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

    pub fn get_visibility(&self, alt: Length) -> Option<Length> {
        for layer in self.get_cloud_layers() {
            if alt >= layer.altitude_min
                && alt <= layer.altitude_max
                && matches!(
                    layer.coverage,
                    CloudCoverage::Scattered | CloudCoverage::Broken | CloudCoverage::Overcast
                )
            {
                return Some(Length::zero());
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
                    x if (0.9..f32::MAX).contains(&x) => CloudCoverage::Overcast,
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

    pub fn get_visibility(&self) -> Option<Length> {
        if self.preset.precipitation_power <= 0.0 {
            return None;
        }

        match self.preset.precipitation_power {
            x if (0.0..0.3).contains(&x) => Some(Length::new::<meter>(5_000)),
            x if (0.3..0.5).contains(&x) => Some(Length::new::<meter>(4_000)),
            x if (0.5..0.6).contains(&x) => Some(Length::new::<meter>(3_000)),
            x if (0.6..0.7).contains(&x) => Some(Length::new::<meter>(2_500)),
            x if (0.7..0.8).contains(&x) => Some(Length::new::<meter>(2_000)),
            x if (0.8..0.9).contains(&x) => Some(Length::new::<meter>(1_500)),
            x if (0.9..0.97).contains(&x) => Some(Length::new::<meter>(1_000)),
            x if (1.0..f64::MAX).contains(&x) => Some(Length::new::<meter>(700)),
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
            altitude_min: if self.base < Length::new::<foot>(1_000) {
                Length::zero()
            } else {
                self.base - Length::new::<foot>(200)
            },
            altitude_max: self.base + self.thickness - Length::new::<foot>(200),
        }]
    }

    pub fn get_weather_conditions(&self) -> Vec<WeatherCondition> {
        match self.iprecptns {
            1 => vec![WeatherCondition::Rain],
            2 => vec![WeatherCondition::Rain, WeatherCondition::Thunderstorm],
            _ => vec![],
        }
    }

    pub fn get_visibility(&self) -> Option<Length> {
        match self.iprecptns {
            1 => Some(Length::new::<meter>(7_400)),
            2 => Some(Length::new::<meter>(1_200)),
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
