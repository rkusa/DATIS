use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::station::{LatLngPosition, Position};
use futures::channel::oneshot::{channel, Receiver, Sender};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Success(Value),
    Error(String),
}

#[derive(Debug)]
pub struct PendingRequest {
    method: String,
    params: Option<Value>,
    tx: Sender<Response>,
}

#[derive(Debug)]
pub struct MissionRpcInner {
    queue: VecDeque<PendingRequest>,
    clouds: Option<Clouds>,
    fog_thickness: u32,  // in m
    fog_visibility: u32, // in m
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Clouds {
    pub base: u32, // in m
    pub density: u32,
    pub thickness: u32,
    pub iprecptns: u32,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct WeatherInfo {
    pub clouds: Option<Clouds>,
    pub visibility: Option<u32>, // in m
    pub wind_speed: f64,         // in m/s
    pub wind_dir: f64,           // in degrees (the direction the wind is coming from)
    pub temperature: f64,        // in °C
    pub pressure_qnh: f64,       // in N/m2
    pub pressure_qfe: f64,       // in N/m2
}

#[derive(Clone)]
pub struct MissionRpc(Arc<Mutex<MissionRpcInner>>);

impl MissionRpc {
    pub fn new(
        clouds: Option<Clouds>,
        fog_thickness: u32,
        fog_visibility: u32,
    ) -> Result<Self, anyhow::Error> {
        Ok(MissionRpc(Arc::new(Mutex::new(MissionRpcInner {
            queue: VecDeque::new(),
            clouds,
            fog_thickness,
            fog_visibility,
        }))))
    }

    pub fn try_next(&self) -> Option<PendingRequest> {
        if let Ok(mut inner) = self.0.try_lock() {
            inner.queue.pop_front()
        } else {
            None
        }
    }

    pub async fn get_weather_at(&self, pos: &Position) -> Result<WeatherInfo, anyhow::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Data {
            wind_speed: f64,
            wind_dir: f64,
            temp: f64,
            pressure: f64,
        }

        let (rx, clouds, visibility) = {
            let (req, rx) = PendingRequest::new(
                "get_weather",
                Some(json!({ "x": pos.x, "y": pos.y, "alt": 0})),
            );

            let mut inner = self.0.lock().unwrap();
            inner.queue.push_back(req);

            let clouds = inner.clouds.clone();

            let visibility = if inner.fog_thickness > 200 {
                Some(inner.fog_visibility)
            } else {
                None
            };

            (rx, clouds, visibility)
        };

        let data: Data = match rx.await? {
            Response::Success(v) => serde_json::from_value(v)?,
            Response::Error(err) => {
                return Err(anyhow!("failed to get weather: {}", err));
            }
        };
        let pressure_qnh = data.pressure;

        let rx = {
            let (req, rx) = PendingRequest::new(
                "get_weather",
                Some(json!({ "x": pos.x, "y": pos.y, "alt": pos.alt})),
            );
            let mut inner = self.0.lock().unwrap();
            inner.queue.push_back(req);
            rx
        };

        let data: Data = match rx.await? {
            Response::Success(v) => serde_json::from_value(v)?,
            Response::Error(err) => {
                return Err(anyhow!("failed to get weather: {}", err));
            }
        };

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
        })
    }

    pub async fn get_unit_position(&self, name: &str) -> Result<Option<Position>, anyhow::Error> {
        let rx = {
            let (req, rx) = PendingRequest::new("get_unit_position", Some(json!({ "name": name })));
            let mut inner = self.0.lock().unwrap();
            inner.queue.push_back(req);
            rx
        };

        match rx.await? {
            Response::Success(v) => Ok(Some(serde_json::from_value(v)?)),
            Response::Error(err) => {
                error!("failed to get position of unit {}: {}", name, err);
                Ok(None)
            }
        }
    }

    pub async fn get_unit_heading(&self, name: &str) -> Result<Option<f64>, anyhow::Error> {
        let rx = {
            let (req, rx) = PendingRequest::new("get_unit_heading", Some(json!({ "name": name })));
            let mut inner = self.0.lock().unwrap();
            inner.queue.push_back(req);
            rx
        };

        match rx.await? {
            Response::Success(v) => Ok(Some(serde_json::from_value(v)?)),
            Response::Error(err) => {
                error!("failed to get heading of unit {}: {}", name, err);
                Ok(None)
            }
        }
    }

    async fn get_abs_time(&self) -> Result<f64, anyhow::Error> {
        let rx = {
            let (req, rx) = PendingRequest::new("get_abs_time", None);
            let mut inner = self.0.lock().unwrap();
            inner.queue.push_back(req);
            rx
        };

        match rx.await? {
            Response::Success(v) => Ok(serde_json::from_value(v)?),
            Response::Error(err) => Err(anyhow!("failed to get abs time: {}", err)),
        }
    }

    pub async fn get_mission_hour(&self) -> Result<u16, anyhow::Error> {
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

    pub async fn to_lat_lng(&self, pos: &Position) -> Result<LatLngPosition, anyhow::Error> {
        let rx = {
            let (req, rx) = PendingRequest::new(
                "to_lat_lng",
                Some(json!({ "x": pos.x, "y": pos.y, "alt": pos.alt})),
            );
            let mut inner = self.0.lock().unwrap();
            inner.queue.push_back(req);
            rx
        };

        match rx.await? {
            Response::Success(v) => Ok(serde_json::from_value(v)?),
            Response::Error(err) => Err(anyhow!("failed to get abs time: {}", err)),
        }
    }
}

impl PendingRequest {
    pub fn new(method: &str, params: Option<Value>) -> (Self, Receiver<Response>) {
        let (tx, rx) = channel();
        (
            PendingRequest {
                method: method.to_string(),
                params,
                tx,
            },
            rx,
        )
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn take_params(&mut self) -> Option<Value> {
        self.params.take()
    }

    pub fn receive(self, res: Response) {
        let _ = self.tx.send(res);
    }
}
