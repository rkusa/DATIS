use std::ops::Deref;
use std::sync::Arc;

use crate::station::{LatLngPosition, Position};
use crate::weather::{Clouds, WeatherInfo};
use dcs_module_ipc::Error;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::Mutex;
use uom::si::angle::degree;
use uom::si::f64::{Angle, Pressure, ThermodynamicTemperature as Temperature, Velocity};
use uom::si::i32::Length;

pub struct MissionRpcInner {
    ipc: dcs_module_ipc::IPC<()>,
    clouds: Mutex<Option<Clouds>>,
}

#[derive(Clone)]
pub struct MissionRpc(Arc<MissionRpcInner>);

impl MissionRpc {
    pub async fn get_weather_at(&self, pos: &Position) -> Result<WeatherInfo, Error> {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Data {
            #[serde(deserialize_with = "crate::de::from_meter_per_second")]
            wind_speed: Velocity,
            #[serde(deserialize_with = "crate::de::from_radian")]
            wind_dir: Angle,
            #[serde(deserialize_with = "crate::de::from_degree_celcius")]
            temp: Temperature,
            #[serde(deserialize_with = "crate::de::from_pascal")]
            pressure: Pressure,
            #[serde(deserialize_with = "crate::de::from_meter")]
            fog_thickness: Length,
            #[serde(deserialize_with = "crate::de::from_meter")]
            fog_visibility: Length,
            dust_density: i32,
        }

        // first, get weather at sea level
        let data: Data = self
            .0
            .ipc
            .request(
                "get_weather",
                Some(json!({ "x": pos.x, "y": pos.y, "alt": 0})),
            )
            .await?;
        // ... to retrieve the QNH
        let pressure_sealevel = data.pressure;

        // then get weather at actual altitude
        let data: Data = self
            .0
            .ipc
            .request(
                "get_weather",
                Some(json!({ "x": pos.x, "y": pos.y, "alt": pos.alt})),
            )
            .await?;

        // convert to degrees and rotate wind direction
        let mut wind_dir = data.wind_dir - Angle::new::<degree>(180.0);

        // normalize wind direction
        while wind_dir < Angle::new::<degree>(0.0) {
            wind_dir += Angle::new::<degree>(360.0);
        }

        let clouds = {
            let mut clouds = self.0.clouds.lock().await;
            if clouds.is_none() {
                *clouds = Some(self.get_clouds().await?);
            }
            clouds.clone().unwrap()
        };

        Ok(WeatherInfo {
            clouds: Some(clouds),
            wind_speed: data.wind_speed,
            wind_dir,
            temperature: data.temp,
            pressure_sealevel,
            pressure_groundlevel: data.pressure,
            fog_thickness: data.fog_thickness,
            fog_visibility: data.fog_visibility,
            dust_density: data.dust_density,
            position: pos.clone(),
        })
    }

    pub async fn get_clouds(&self) -> Result<Clouds, Error> {
        let clouds: Clouds = self.0.ipc.request("get_clouds", None::<()>).await?;

        Ok(clouds)
    }

    pub async fn get_unit_position(&self, name: &str) -> Result<Position, Error> {
        self.0
            .ipc
            .request("get_unit_position", Some(json!({ "name": name })))
            .await
    }

    pub async fn get_unit_heading(&self, name: &str) -> Result<Option<f64>, Error> {
        self.0
            .ipc
            .request("get_unit_heading", Some(json!({ "name": name })))
            .await
    }

    async fn get_abs_time(&self) -> Result<f64, Error> {
        self.0.ipc.request::<(), _>("get_abs_time", None).await
    }

    pub async fn get_mission_start_date(&self) -> Result<time::Date, Error> {
        let date: String = self
            .0
            .ipc
            .request::<(), _>("get_mission_start_date", None)
            .await?;
        let format =
            time::format_description::parse("[year]-[month padding:none]-[day padding:none]")
                .map_err(|err| Error::Script {
                    kind: None,
                    message: err.to_string(),
                })?;
        time::Date::parse(&date, &format).map_err(|err| Error::Script {
            kind: None,
            message: err.to_string(),
        })
    }

    pub async fn get_mission_hour(&self) -> Result<u16, Error> {
        let mut time = self.get_abs_time().await?;
        let mut h = 0;

        while time >= 86_400.0 {
            time -= 86_400.0;
            // ignore days
        }

        while time >= 3_600.0 {
            time -= 3_600.0;
            h += 1;
        }

        Ok(h)
    }

    pub async fn to_lat_lng(&self, pos: &Position) -> Result<LatLngPosition, Error> {
        self.0
            .ipc
            .request(
                "to_lat_lng",
                Some(json!({ "x": pos.x, "y": pos.y, "alt": pos.alt})),
            )
            .await
    }
}

impl Default for MissionRpc {
    fn default() -> Self {
        MissionRpc(Arc::new(MissionRpcInner {
            ipc: dcs_module_ipc::IPC::default(),
            clouds: Mutex::new(None),
        }))
    }
}

impl Deref for MissionRpc {
    type Target = dcs_module_ipc::IPC<()>;

    fn deref(&self) -> &Self::Target {
        &self.0.ipc
    }
}
