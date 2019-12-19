const RADIO_RECEIVE_PORT: u16 = 9084;

use futures::channel::mpsc;

use tokio::net::{UdpSocket};

use srs::message::GameMessage;

pub async fn dcs_control(mut tx: mpsc::Sender<GameMessage>)
    -> Result<(), anyhow::Error>
{
    let mut socket = UdpSocket::bind(format!("127.0.0.1:{}", RADIO_RECEIVE_PORT))
        .await
        .expect("Failed to create dcs control socket");

    println!("Starting dcs control task");

    loop {
        let mut buf = [0u8; 2048];
        let amount = socket.recv(&mut buf).await
            .expect("Failed to read bytes from socket");

        let decoded = serde_json::from_slice::<GameMessage>(&buf[0..amount]);
        match decoded {
            Ok(message) => {
                match tx.try_send(message) {
                    Ok(()) => {},
                    Err(e) => {
                        warn!("Game message sending failed: {:?}", e)
                    }
                }
            }
            Err(e) => {
                warn!("Failed to decode game message: {:?}", e);
            }
        }
    }
}
