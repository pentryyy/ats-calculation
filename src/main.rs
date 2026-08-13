mod dto;
mod utils;

use crate::dto::request::audio::AudioData;
use crate::dto::response::angle::AngleData;
use std::net::UdpSocket;

fn main() -> std::io::Result<()> {
    let server_socket = UdpSocket::bind("127.0.0.1:8080")?;
    println!("Сервер слушает на порту 8080");

    let mut buf = [0u8; 1024];

    let client_socket = UdpSocket::bind("127.0.0.1:0")?;

    let audio = AudioData {
        mic1: vec![1, 2, 3, 4],
        mic2: vec![5, 6, 7, 8],
    };

    let audio_bytes = bincode::serialize(&audio).unwrap();
    client_socket.send_to(&audio_bytes, "127.0.0.1:8080")?;
    println!("Клиент отправил AudioData: {:?} байт", audio_bytes.len());

    let (len, src_addr) = server_socket.recv_from(&mut buf)?;
    let received_data = &buf[..len];

    let received_audio: AudioData = bincode::deserialize(received_data).unwrap();
    println!("Сервер получил AudioData: {:?}", received_audio);

    let angle = AngleData { angle: 42.0 };
    let angle_bytes = bincode::serialize(&angle).unwrap();
    server_socket.send_to(&angle_bytes, src_addr)?;
    println!("Сервер отправил AngleData: {:?} байт", angle_bytes.len());

    let (len, _) = client_socket.recv_from(&mut buf)?;
    let received_data = &buf[..len];
    let received_angle: AngleData = bincode::deserialize(received_data).unwrap();
    println!("Клиент получил AngleData: {:?}", received_angle);

    Ok(())
}
