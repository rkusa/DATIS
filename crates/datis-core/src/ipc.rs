use std::ops::Deref;
use std::sync::Arc;

use crate::station::{LatLngPosition, Position};
use crate::weather::{Clouds, WeatherInfo};
use dcs_module_ipc::Error;
use serde::Deserialize;
use serde_json::json;

pub struct MissionRpcInner {
    ipc: dcs_module_ipc::IPC<()>,
    clouds: Option<Clouds>,
    fog_thickness: u32,  // in m
    fog_visibility: u32, // in m
}

#[derive(Clone)]
pub struct MissionRpc(Arc<MissionRpcInner>);

impl MissionRpc {
    pub fn new(clouds: Option<Clouds>, fog_thickness: u32, fog_visibility: u32) -> Self {
        MissionRpc(Arc::new(MissionRpcInner {
            ipc: dcs_module_ipc::IPC::new(),
            clouds,
            fog_thickness,
            fog_visibility,
        }))
    }

    pub async fn get_weather_at(&self, pos: &Position) -> Result<WeatherInfo, Error> {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Data {
            wind_speed: f64,
            wind_dir: f64,
            temp: f64,
            pressure: f64,
        }

        let clouds = self.0.clouds.clone();

        let visibility = if self.0.fog_thickness > 200 {
            Some(self.0.fog_visibility)
        } else {
            None
        };

        let data: Data = self
            .0
            .ipc
            .request(
                "get_weather",
                Some(json!({ "x": pos.x, "y": pos.y, "alt": 0})),
            )
            .await?;
        let pressure_qnh = data.pressure;

        let data: Data = self
            .0
            .ipc
            .request(
                "get_weather",
                Some(json!({ "x": pos.x, "y": pos.y, "alt": pos.alt})),
            )
            .await?;

        // convert to degrees and rotate wind direction
        let mut wind_dir = data.wind_dir.to_degrees() - 180.0;

        // normalize wind direction
        while wind_dir < 0.0 {
            wind_dir += 360.0;
        }

        Ok(WeatherInfo {
            clouds,
            visibility,
            wind_speed: data.wind_speed,
            wind_dir,
            temperature: data.temp,
            pressure_qnh,
            pressure_qfe: data.pressure,
            position: pos.clone(),
        })
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

impl Deref for MissionRpc {
    type Target = dcs_module_ipc::IPC<()>;

    fn deref(&self) -> &Self::Target {
        &self.0.ipc
    }
}
