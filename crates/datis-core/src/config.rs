use serde::{Deserialize, Serialize};

use crate::tts::TextToSpeechProvider;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub default_voice: TextToSpeechProvider,
    pub gcloud: Option<GcloudConfig>,
    pub aws: Option<AwsConfig>,
    pub azure: Option<AzureConfig>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct AzureConfig {
    pub key: String,
    pub region: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default_voice: TextToSpeechProvider::default(),
            gcloud: None,
            aws: None,
            azure: None,
            srs_port: default_srs_port(),
            debug: false,
        }
    }
}

fn default_srs_port() -> u16 {
    5002
}
