const RADIO_RECEIVE_PORT: u16 = 9084;

use tokio::net::{UdpSocket};

pub async fn dcs_control() -> ! {
    let mut socket = UdpSocket::bind(format!("127.0.0.1:{}", RADIO_RECEIVE_PORT))
        .await
        .expect("Failed to create dcs control socket");

    println!("Starting dcs control task");

    loop {
        println!("In main loop");
        let mut buf = [0u8; 2048];
        let amount = socket.recv(&mut buf).await
            .expect("Failed to read bytes from socket");

        println!("Got {} bytes from dcs socket", amount);
        println!("{}", String::from_utf8_lossy(&buf[0..amount]));
    }
}
