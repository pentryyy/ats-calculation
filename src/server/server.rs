use crate::config::config::{AppConfig, AudioConfig};
use crate::dto::request::audio::AudioData;
use crate::dto::response::angle::AngleData;
use crate::services::socket::{PacketSender, SocketService};
use crate::services::spectral_vad::{SpectralVAD, VADDetector};
use crate::utils::calculate_angle::calculate_angle;
use anyhow::Result;
use env_logger::Builder;
use log::info;
use std::net::SocketAddr;

pub fn run(cfg: &AppConfig) -> Result<()> {
    Builder::new().filter_level(cfg.log_level()).init();

    let mut server = SocketService::bind(cfg.addr())?;
    let mut vad1 = SpectralVAD::new(cfg.vad.clone());
    let mut vad2 = SpectralVAD::new(cfg.vad.clone());

    info!("Сервер слушает на {}", cfg.addr());

    let mut recv_buf = cfg.recv_buf();
    let mut send_buf = cfg.send_buf();

    loop {
        let (received_audio, src_addr) = server.recv_from::<AudioData>(&mut recv_buf)?;
        process_packet(
            &mut vad1,
            &mut vad2,
            &received_audio,
            &mut server,
            src_addr,
            &cfg.audio,
            &mut send_buf,
        )?;
    }
}

fn process_packet<V1, V2, S>(
    vad1: &mut V1,
    vad2: &mut V2,
    audio_dto: &AudioData,
    server: &mut S,
    src_addr: SocketAddr,
    audio_cfg: &AudioConfig,
    buf: &mut Vec<u8>,
) -> Result<()>
where
    V1: VADDetector + Send,
    V2: VADDetector + Send,
    S: PacketSender,
{
    info!("Сервер получил AudioData от {}", src_addr);

    let (has_speech1, has_speech2) = rayon::join(
        || vad1.detect_speech(&audio_dto.mic1, audio_cfg.sample_rate),
        || vad2.detect_speech(&audio_dto.mic2, audio_cfg.sample_rate),
    );

    if !(has_speech1 || has_speech2) {
        info!("Речи нет, пропускаем");
        return Ok(());
    }

    info!(
        "Речь обнаружена (mic1: {}, mic2: {})",
        has_speech1, has_speech2
    );
    let angle = calculate_angle(
        &audio_dto.mic1,
        &audio_dto.mic2,
        audio_cfg.sample_rate,
        audio_cfg.mic_distance,
    );

    let angle_data = AngleData { angle };
    server.send_to(&angle_data, src_addr, buf)?;
    info!("Сервер отправил AngleData: {:?}", angle_data);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::request::audio::AudioData;
    use crate::dto::response::angle::AngleData;
    use crate::services::socket::PacketSender;
    use crate::services::spectral_vad::VADDetector;
    use serde::Serialize;
    use std::collections::VecDeque;
    use std::net::{Ipv4Addr, SocketAddr};

    struct MockVAD {
        results: VecDeque<bool>,
    }

    impl MockVAD {
        fn new(results: Vec<bool>) -> Self {
            MockVAD {
                results: VecDeque::from(results),
            }
        }
    }

    impl VADDetector for MockVAD {
        fn detect_speech(&mut self, _frame: &[i16], _sample_rate: u32) -> bool {
            self.results.pop_front().unwrap_or(false)
        }
    }

    struct MockSocket {
        last_payload: Vec<u8>,
        last_addr: Option<SocketAddr>,
        should_fail: bool,
    }

    impl MockSocket {
        fn new() -> Self {
            MockSocket {
                last_payload: Vec::new(),
                last_addr: None,
                should_fail: false,
            }
        }

        fn with_failure() -> Self {
            let mut s = Self::new();
            s.should_fail = true;
            s
        }

        fn get_last_payload<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
            if self.last_payload.is_empty() {
                return None;
            }
            bincode::deserialize(&self.last_payload).ok()
        }

        fn get_last_addr(&self) -> Option<SocketAddr> {
            self.last_addr
        }
    }

    impl PacketSender for MockSocket {
        fn send_to<T: Serialize>(
            &mut self,
            data: &T,
            addr: SocketAddr,
            buf: &mut Vec<u8>,
        ) -> Result<usize> {
            if self.should_fail {
                return Err(anyhow::anyhow!("send failed"));
            }
            buf.clear();
            bincode::serialize_into(&mut *buf, data)?;
            self.last_payload = buf.clone();
            self.last_addr = Some(addr);
            Ok(buf.len())
        }
    }

    fn create_audio_data(mic1: Vec<i16>, mic2: Vec<i16>) -> AudioData {
        AudioData { mic1, mic2 }
    }

    fn test_addr() -> SocketAddr {
        SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8080)
    }

    fn test_audio_cfg() -> AudioConfig {
        AudioConfig {
            sample_rate: 16000,
            mic_distance: 0.2,
        }
    }

    #[test]
    fn test_process_packet_no_speech() {
        let mut vad1 = MockVAD::new(vec![false]);
        let mut vad2 = MockVAD::new(vec![false]);
        let mut socket = MockSocket::new();
        let audio = create_audio_data(vec![0; 512], vec![0; 512]);
        let mut buf = Vec::new();

        let result = process_packet(
            &mut vad1,
            &mut vad2,
            &audio,
            &mut socket,
            test_addr(),
            &test_audio_cfg(),
            &mut buf,
        );

        assert!(result.is_ok());
        assert!(socket.get_last_payload::<AngleData>().is_none());
    }

    #[test]
    fn test_process_packet_with_speech() {
        let mut vad1 = MockVAD::new(vec![true]);
        let mut vad2 = MockVAD::new(vec![false]);
        let mut socket = MockSocket::new();
        let mic1: Vec<i16> = (0..512)
            .map(|i| {
                let val =
                    10000.0 * (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 16000.0).sin();
                val as i16
            })
            .collect();
        let mic2 = mic1.clone();
        let audio = create_audio_data(mic1, mic2);
        let mut buf = Vec::new();

        let result = process_packet(
            &mut vad1,
            &mut vad2,
            &audio,
            &mut socket,
            test_addr(),
            &test_audio_cfg(),
            &mut buf,
        );

        assert!(result.is_ok());
        let angle_data: Option<AngleData> = socket.get_last_payload();
        assert!(angle_data.is_some());
        let angle = angle_data.unwrap().angle;
        assert!((angle - 0.0).abs() < 1.0);
        assert_eq!(socket.get_last_addr(), Some(test_addr()));
    }

    #[test]
    fn test_process_packet_send_error() {
        let mut vad1 = MockVAD::new(vec![true]);
        let mut vad2 = MockVAD::new(vec![false]);
        let mut socket = MockSocket::with_failure();
        let audio = create_audio_data(vec![0; 512], vec![0; 512]);
        let mut buf = Vec::new();

        let result = process_packet(
            &mut vad1,
            &mut vad2,
            &audio,
            &mut socket,
            test_addr(),
            &test_audio_cfg(),
            &mut buf,
        );

        assert!(result.is_err());
    }
}
