mod config;
mod dto;
mod services;
mod utils;

use crate::config::config::Config;
use crate::dto::request::audio::AudioData;
use crate::dto::response::angle::AngleData;
use crate::services::socket::SocketService;
use anyhow::Result;

fn main() -> Result<()> {
    let cfg = Config::load()?;
    let server = SocketService::bind(cfg.addr())?;
    println!("Сервер слушает на {}", cfg.addr());

    let client = SocketService::bind("127.0.0.1:0")?;

    let mut buf = cfg.buf();
    let addr = cfg.addr().parse()?;

    let audio = AudioData {
        mic1: vec![1, 2, 3, 4],
        mic2: vec![5, 6, 7, 8],
    };

    client.send_to(&audio, addr)?;
    println!("Клиент отправил AudioData: {:?}", audio);

    let (received_audio, src_addr) = server.recv_from::<AudioData>(&mut buf)?;
    println!("Сервер получил AudioData: {:?}", received_audio);

    let angle = AngleData { angle: 42.0 };
    server.send_to(&angle, src_addr)?;
    println!("Сервер отправил AngleData: {:?}", angle);

    let (received_angle, _) = client.recv_from::<AngleData>(&mut buf)?;
    println!("Клиент получил AngleData: {:?}", received_angle);

    Ok(())
}
