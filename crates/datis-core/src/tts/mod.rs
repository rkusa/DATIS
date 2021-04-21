pub mod aws;
pub mod gcloud;
pub mod win;
pub mod azure;

use std::fmt;
use std::str::FromStr;

#[derive(PartialEq, Clone)]
pub enum TextToSpeechProvider {
    GoogleCloud { voice: gcloud::VoiceKind },
    AmazonWebServices { voice: aws::VoiceKind },
    Windows { voice: Option<win::VoiceKind> },
    AzureCognitiveServices { voice: azure::VoiceKind },
}

#[derive(Clone)]
pub enum TextToSpeechConfig {
    GoogleCloud(gcloud::GoogleCloudConfig),
    AmazonWebServices(aws::AmazonWebServicesConfig),
    Windows(win::WindowsConfig),
    AzureCognitiveServices(azure::AzureCognitiveServicesConfig),
}

impl Default for TextToSpeechProvider {
    fn default() -> Self {
        TextToSpeechProvider::Windows { voice: None }
    }
}

impl fmt::Debug for TextToSpeechProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            TextToSpeechProvider::GoogleCloud { voice } => {
                write!(f, "Google Cloud (Voice: {:?})", voice)
            }
            TextToSpeechProvider::AmazonWebServices { voice } => {
                write!(f, "Amazon Web Services (Voice: {:?})", voice)
            }
            TextToSpeechProvider::AzureCognitiveServices { voice } => {
                write!(f, "Azure Cognitive Services (Voice: {:?})", voice)
            }
            TextToSpeechProvider::Windows { voice } => write!(
                f,
                "Windows built-in TTS (Voice: {:?})",
                voice.as_ref().map(|v| &**v).unwrap_or_else(|| "Default")
            ),
        }
    }
}

impl FromStr for TextToSpeechProvider {
    type Err = TextToSpeechProviderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v: Vec<&str> = s.splitn(2, ':').collect();
        match *v.as_slice() {
            [prefix, voice] => match prefix {
                "GC" | "gc" => {
                    return Ok(TextToSpeechProvider::GoogleCloud {
                        voice: gcloud::VoiceKind::from_str(voice)
                            .map_err(TextToSpeechProviderError::Voice)?,
                    })
                }
                "AWS" | "aws" => {
                    return Ok(TextToSpeechProvider::AmazonWebServices {
                        voice: aws::VoiceKind::from_str(voice)
                            .map_err(TextToSpeechProviderError::Voice)?,
                    })
                }
                "AZURE" | "azure" => {
                    return Ok(TextToSpeechProvider::AzureCognitiveServices {
                        voice: azure::VoiceKind::from_str(voice)
                            .map_err(TextToSpeechProviderError::Voice)?,
                    })
                }
                "WIN" | "win" => {
                    return Ok(TextToSpeechProvider::Windows {
                        voice: Some(
                            win::VoiceKind::from_str(voice)
                                .map_err(TextToSpeechProviderError::Voice)?,
                        ),
                    })
                }
                _ => {}
            },
            [voice] if !voice.is_empty() => {
                if voice == "WIN" || voice == "win" {
                    return Ok(TextToSpeechProvider::Windows { voice: None });
                } else {
                    return Ok(TextToSpeechProvider::GoogleCloud {
                        voice: gcloud::VoiceKind::from_str(voice)
                            .map_err(TextToSpeechProviderError::Voice)?,
                    });
                }
            }
            _ => {}
        }

        Err(TextToSpeechProviderError::Provider(s.to_string()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TextToSpeechProviderError {
    #[error("Invalid default voice `{0}`")]
    Provider(String),
    #[error("Invalid default voice: `{0}`")]
    Voice(serde_json::Error),
}

impl<'de> serde::Deserialize<'de> for TextToSpeechProvider {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl serde::Serialize for TextToSpeechProvider {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&match self {
            TextToSpeechProvider::GoogleCloud { voice } => {
                format!("GC:{}", voice)
            }
            TextToSpeechProvider::AmazonWebServices { voice } => {
                format!("AWS:{}", voice)
            }
            TextToSpeechProvider::AzureCognitiveServices { voice } => {
                format!("AZURE:{}", voice)
            }
            TextToSpeechProvider::Windows { voice } => {
                if let Some(voice) = voice {
                    format!("WIN:{}", voice)
                } else {
                    "WIN".to_string()
                }
            }
        })
    }
}

#[cfg(test)]
mod test {
    mod tts_provider_from_str {
        use std::str::FromStr;

        use crate::tts::{aws, gcloud, azure, TextToSpeechProvider};

        #[test]
        fn err_when_invalid() {
            assert!(TextToSpeechProvider::from_str("").is_err())
        }

        #[test]
        fn err_on_unknown_prefix() {
            assert!(TextToSpeechProvider::from_str("UNK:foobar").is_err())
        }

        #[test]
        fn no_prefix_defaults_to_gcloud() {
            assert_eq!(
                TextToSpeechProvider::from_str("en-US-Wavenet-A").unwrap(),
                TextToSpeechProvider::GoogleCloud {
                    voice: gcloud::VoiceKind::EnUsWavenetA
                }
            )
        }

        #[test]
        fn prefix_gc() {
            assert_eq!(
                TextToSpeechProvider::from_str("GC:en-US-Wavenet-B").unwrap(),
                TextToSpeechProvider::GoogleCloud {
                    voice: gcloud::VoiceKind::EnUsWavenetB
                }
            )
        }

        #[test]
        fn gc_en_gp() {
            assert_eq!(
                TextToSpeechProvider::from_str("GC:en-GB-Standard-A").unwrap(),
                TextToSpeechProvider::GoogleCloud {
                    voice: gcloud::VoiceKind::EnGbStandardA
                }
            )
        }

        #[test]
        fn prefix_aws() {
            assert_eq!(
                TextToSpeechProvider::from_str("AWS:Brian").unwrap(),
                TextToSpeechProvider::AmazonWebServices {
                    voice: aws::VoiceKind::Brian
                }
            )
        }

        #[test]
        fn prefix_azure() {
            assert_eq!(
                TextToSpeechProvider::from_str("AZURE:en-US-AriaRUS").unwrap(),
                TextToSpeechProvider::AzureCognitiveServices {
                    voice: azure::VoiceKind::AriaRUS
                }
            )
        }
    }
}
