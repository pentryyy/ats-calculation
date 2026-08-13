mod config;
mod dto;
mod server;
mod services;
mod utils;

use crate::config::config::AppConfig;
use crate::dto::request::audio::AudioData;
use crate::services::socket::SocketService;

use anyhow::Result;
use rand::Rng;
use std::thread;
use std::time::Duration;
use crate::server::server::run;

fn main() -> Result<()> {
    let cfg = AppConfig::load()?;

    // thread::spawn(move || {
    //     let client = SocketService::bind("127.0.0.1:0").unwrap();
    //     let addr = cfg.addr().parse().unwrap();
    //     let mut rng = rand::thread_rng();
    //     let mut speech = rng.gen_bool(0.5);
    //     let mut remaining_time = rng.gen_range(5.0..20.0);
    //     let packet_duration = cfg.vad.fft_size as f32 / cfg.audio.sample_rate as f32;
    //
    //     loop {
    //         let audio = create_audio(&cfg, speech);
    //         if let Err(e) = client.send_to(&audio, addr) {
    //             eprintln!("Ошибка отправки: {}", e);
    //         }
    //
    //         remaining_time -= packet_duration;
    //         if remaining_time <= 0.0 {
    //             speech = !speech;
    //             remaining_time = rng.gen_range(5.0..20.0);
    //             println!("Переключение на {}", if speech { "речь" } else { "тишину" });
    //         }
    //
    //         thread::sleep(Duration::from_millis(10));
    //     }
    // });
    //
    // Ok(())

    if let Err(e) = run(&cfg) {
        std::process::exit(1);
    }

    Ok(())
}

// fn create_audio(cfg: &AppConfig, speech: bool) -> AudioData {
//     let fft_size = cfg.vad.fft_size;
//     let sample_rate = cfg.audio.sample_rate;
//
//     if !speech {
//         return AudioData {
//             mic1: vec![0; fft_size],
//             mic2: vec![0; fft_size],
//         };
//     }
//
//     let freq_hz = 500.0;
//     let amplitude = 20000.0;
//     let mic1: Vec<i16> = (0..fft_size)
//         .map(|i| {
//             let t = i as f32 / sample_rate as f32;
//             (amplitude * (2.0 * std::f32::consts::PI * freq_hz * t).sin()) as i16
//         })
//         .collect();
//     let delay_sec = 0.0001;
//     let mic2: Vec<i16> = (0..fft_size)
//         .map(|i| {
//             let t = i as f32 / sample_rate as f32 - delay_sec;
//             if t >= 0.0 {
//                 (amplitude * (2.0 * std::f32::consts::PI * freq_hz * t).sin()) as i16
//             } else {
//                 0
//             }
//         })
//         .collect();
//
//     AudioData { mic1, mic2 }
// }
