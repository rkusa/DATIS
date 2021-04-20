use crate::tts::TextToSpeechProvider;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub default_voice: TextToSpeechProvider,
    pub gcloud: Option<GcloudConfig>,
    pub aws: Option<AwsConfig>,
    #[serde(default = "default_srs_port")]
    pub srs_port: u16,
    #[serde(default)]
    pub debug: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GcloudConfig {
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AwsConfig {
    pub key: String,
    pub secret: String,
    pub region: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default_voice: TextToSpeechProvider::default(),
            gcloud: None,
            aws: None,
            srs_port: default_srs_port(),
            debug: false,
        }
    }
}

fn default_srs_port() -> u16 {
    5002
}
