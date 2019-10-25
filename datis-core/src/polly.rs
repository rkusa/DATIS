//use std
use crate::error::Error;
use futures::future::Future;
use std::default::Default;
use std::env;

//use rusoto
use rusoto_core::Region;
use rusoto_credential::{EnvironmentProvider, ProvideAwsCredentials};
use rusoto_polly::{Polly, PollyClient, SynthesizeSpeechInput};

pub fn polly_tts(
    tts: &str,
    voice_id: &str,
    aws_access_key: &str,
    aws_secret_key: &str,
    aws_region: &str,
) -> Result<Vec<i16>, Error> {
    //credentials
    env::set_var("AWS_ACCESS_KEY_ID", aws_access_key);
    env::set_var("AWS_SECRET_ACCESS_KEY", aws_secret_key);
    //this is new line
    env::set_var("AWS_REGION", aws_region);

    //put them into a class
    let _creds = EnvironmentProvider::default().credentials().wait().unwrap();

    //Add output format
    let output_format = "pcm";

    //Add text type
    let txt_type: Option<String> = Some(String::from("ssml"));

    //log request string
    debug!("Sending polly this ssml string {}.", String::from(tts));

    //Build text_to_speech request
    let _text_to_speech_request = SynthesizeSpeechInput {
        language_code: Option::default(),
        lexicon_names: Option::default(),
        output_format: String::from(output_format),
        sample_rate: Option::default(),
        speech_mark_types: Option::default(),
        text: String::from(tts),
        text_type: txt_type,
        voice_id: String::from(voice_id),
    };

    //build client which seems to default to the credential provider
    //let _polly_client = PollyClient::new(Region::UsEast1);
    //new line to test region env variable
    let _polly_client = PollyClient::new(Region::default());

    //write log
    debug!("Sending request to Amazon.");
    //post request
    let response = _polly_client
        .synthesize_speech(_text_to_speech_request)
        .sync();
    debug!("Got response from Amazon.");

    if response.is_err() {
        Err(Error::PollyTTS(String::from("Polly error!")))
    } else {
        //expose respons
        let unwrapped_response = response.unwrap();
        let audio_stream = unwrapped_response.audio_stream;
        let unwrapped_audio = audio_stream.unwrap();
        let i16_stream = vector_i16(unwrapped_audio);
        Ok(i16_stream)
    }
}

fn vector_i16(byte_stream: bytes::Bytes) -> Vec<i16> {
    let len = byte_stream.len();
    let mut res: Vec<i16> = Vec::new();
    let mut index_pos = 0;
    //hopefully this pushed the bits 8 bits at a time into a stream of u8
    while index_pos < len {
        let this_byte = byte_stream[index_pos];
        let next_byte = byte_stream[index_pos + 1];
        let these_converted = i16::from_le_bytes([this_byte, next_byte]);
        res.push(these_converted);
        index_pos += 2;
    }
    return res;
}
