mod config;
mod dto;
mod services;
mod utils;

use crate::config::config::AppConfig;
use crate::dto::request::audio::AudioData;
use crate::dto::response::angle::AngleData;
use crate::services::socket::SocketService;
use crate::services::spectral_vad::SpectralVAD;
use crate::utils::calculate_angle::calculate_angle;
use anyhow::Result;

fn main() -> Result<()> {
    let cfg = AppConfig::load()?;
    let server = SocketService::bind(cfg.addr())?;
    let mut vad = SpectralVAD::new(cfg.vad.clone());
    println!("Сервер слушает на {}", cfg.addr());

    let client = SocketService::bind("127.0.0.1:0")?;
    let mut buf = cfg.buf();
    let addr = cfg.addr().parse()?;

    let audio = create_audio(&cfg);
    client.send_to(&audio, addr)?;
    println!("Клиент отправил AudioData: {:?}", audio);

    let (received_audio, src_addr) = server.recv_from::<AudioData>(&mut buf)?;
    println!("Сервер получил AudioData: {:?}", received_audio);

    let sample_rate = cfg.audio.sample_rate;
    let mic_distance = cfg.audio.mic_distance;

    let has_speech1 = vad.detect_speech(&received_audio.mic1, sample_rate);
    let has_speech2 = vad.detect_speech(&received_audio.mic2, sample_rate);

    let angle = if has_speech1 || has_speech2 {
        println!(
            "Речь обнаружена (mic1: {}, mic2: {})",
            has_speech1, has_speech2
        );
        calculate_angle(
            &received_audio.mic1,
            &received_audio.mic2,
            sample_rate,
            mic_distance,
        )
    } else {
        println!("Речи нет, отправляем угол 0.0");
        0.0
    };

    let angle_data = AngleData { angle };
    server.send_to(&angle_data, src_addr)?;
    println!("Сервер отправил AngleData: {:?}", angle_data);

    let (received_angle, _) = client.recv_from::<AngleData>(&mut buf)?;
    println!("Клиент получил AngleData: {:?}", received_angle);

    Ok(())
}

fn create_audio(cfg: &AppConfig) -> AudioData {
    let fft_size = cfg.vad.fft_size;
    let sample_rate = cfg.audio.sample_rate;
    let freq_hz = 500.0;
    let amplitude = 20000.0;
    let mic1: Vec<i16> = (0..fft_size)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (amplitude * (2.0 * std::f32::consts::PI * freq_hz * t).sin()) as i16
        })
        .collect();
    let delay_sec = 0.0001;
    let mic2: Vec<i16> = (0..fft_size)
        .map(|i| {
            let t = i as f32 / sample_rate as f32 - delay_sec;
            if t >= 0.0 {
                (amplitude * (2.0 * std::f32::consts::PI * freq_hz * t).sin()) as i16
            } else {
                0
            }
        })
        .collect();

    AudioData { mic1, mic2 }
}
