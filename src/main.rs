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
use std::thread;
use std::time::Duration;
use rand::Rng;

fn main() -> Result<()> {
    let cfg = AppConfig::load()?;
    let server = SocketService::bind(cfg.addr())?;
    let mut vad = SpectralVAD::new(cfg.vad.clone());

    println!("Сервер слушает на {}", cfg.addr());

    let cfg_clone = cfg.clone();
    thread::spawn(move || {
        let client = SocketService::bind("127.0.0.1:0").unwrap();
        let addr = cfg_clone.addr().parse().unwrap();
        let mut rng = rand::thread_rng();
        let mut speech = rng.gen_bool(0.5);
        let mut remaining_time = rng.gen_range(5.0..20.0);
        let packet_duration = cfg_clone.vad.fft_size as f32 / cfg_clone.audio.sample_rate as f32;

        loop {
            let audio = create_audio(&cfg_clone, speech);
            if let Err(e) = client.send_to(&audio, addr) {
                eprintln!("Ошибка отправки: {}", e);
            }

            remaining_time -= packet_duration;
            if remaining_time <= 0.0 {
                speech = !speech;
                remaining_time = rng.gen_range(5.0..20.0);
                println!("Переключение на {}", if speech { "речь" } else { "тишину" });
            }

            thread::sleep(Duration::from_millis(10));
        }
    });

    let sample_rate = cfg.audio.sample_rate;
    let mic_distance = cfg.audio.mic_distance;
    let mut buf = cfg.buf();

    loop {
        match server.recv_from::<AudioData>(&mut buf) {
            Ok((received_audio, src_addr)) => {
                println!("Сервер получил AudioData от {}", src_addr);

                let has_speech1 = vad.detect_speech(&received_audio.mic1, sample_rate);
                let has_speech2 = vad.detect_speech(&received_audio.mic2, sample_rate);

                if !(has_speech1 || has_speech2) {
                    println!("Речи нет, пропускаем");
                    continue;
                }

                println!("Речь обнаружена (mic1: {}, mic2: {})", has_speech1, has_speech2);
                let angle = calculate_angle(
                    &received_audio.mic1,
                    &received_audio.mic2,
                    sample_rate,
                    mic_distance,
                );

                let angle_data = AngleData { angle };
                if let Err(e) = server.send_to(&angle_data, src_addr) {
                    eprintln!("Ошибка отправки угла: {}", e);
                } else {
                    println!("Сервер отправил AngleData: {:?}", angle_data);
                }
            }
            Err(e) => {
                eprintln!("Ошибка приёма: {}", e);
            }
        }
    }
}

fn create_audio(cfg: &AppConfig, speech: bool) -> AudioData {
    let fft_size = cfg.vad.fft_size;
    let sample_rate = cfg.audio.sample_rate;

    if !speech {
        return AudioData {
            mic1: vec![0; fft_size],
            mic2: vec![0; fft_size],
        };
    }

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
