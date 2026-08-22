use crate::config::config::VadConfig;
use num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

const HAMMING_ALPHA: f32 = 0.54;
const HAMMING_BETA: f32 = 0.46;
const EPSILON: f32 = 0.001;

#[inline]
fn freq_to_bin(freq: u32, fft_size: usize, sample_rate: u32) -> usize {
    (freq * fft_size as u32 / sample_rate) as usize
}

pub trait VADDetector {
    fn detect_speech(&mut self, frame: &[i16], sample_rate: u32) -> bool;
}

pub struct SpectralVAD {
    fft: Arc<dyn Fft<f32>>,
    buffer: Vec<Complex<f32>>,
    params: VadConfig,
}

impl SpectralVAD {
    pub fn new(params: VadConfig) -> Self {
        let fft_size = params.fft_size;
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(fft_size);
        Self {
            fft,
            buffer: vec![Complex::new(0.0, 0.0); fft_size],
            params,
        }
    }

    pub fn detect_speech(&mut self, frame: &[i16], sample_rate: u32) -> bool {
        let n = self.buffer.len();
        assert!(frame.len() >= n, "Фрейм короче FFT размера");

        for i in 0..n {
            let window = HAMMING_ALPHA
                - HAMMING_BETA * (2.0 * std::f32::consts::PI * i as f32 / (n - 1) as f32).cos();
            self.buffer[i] = Complex::new(frame[i] as f32 * window, 0.0);
        }

        self.fft.process(&mut self.buffer);

        let low_bin = freq_to_bin(self.params.low_freq, n, sample_rate);
        let speech_bin_start = freq_to_bin(self.params.speech_freq_start, n, sample_rate);
        let speech_bin_end = freq_to_bin(self.params.speech_freq_end, n, sample_rate);

        let mut low_energy = 0.0;
        let mut speech_energy = 0.0;

        for i in 0..n / 2 {
            let mag = self.buffer[i].norm();
            if i < low_bin {
                low_energy += mag;
            } else if i >= speech_bin_start && i <= speech_bin_end {
                speech_energy += mag;
            }
        }

        speech_energy > self.params.speech_energy_threshold
            && (speech_energy / (low_energy + EPSILON)) > self.params.ratio_threshold
    }
}

impl VADDetector for SpectralVAD {
    fn detect_speech(&mut self, frame: &[i16], sample_rate: u32) -> bool {
        self.detect_speech(frame, sample_rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::config::VadConfig;
    use std::f32::consts::PI;

    fn generate_sine(freq: f32, sample_rate: u32, num_samples: usize) -> Vec<i16> {
        let mut v = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let val = (2.0 * PI * freq * t).sin() * 10000.0;
            v.push(val as i16);
        }
        v
    }

    fn default_vad_config() -> VadConfig {
        VadConfig {
            fft_size: 512,
            low_freq: 300,
            speech_freq_start: 300,
            speech_freq_end: 3400,
            speech_energy_threshold: 0.1,
            ratio_threshold: 2.0,
        }
    }

    #[test]
    fn test_freq_to_bin() {
        assert_eq!(freq_to_bin(1000, 512, 16000), 32);
        assert_eq!(freq_to_bin(300, 512, 16000), 9);
        assert_eq!(freq_to_bin(3400, 512, 16000), 108);
    }

    #[test]
    fn test_detect_speech_silence() {
        let params = default_vad_config();
        let mut vad = SpectralVAD::new(params);
        let frame = vec![0i16; 512];
        assert!(!vad.detect_speech(&frame, 16000));
    }

    #[test]
    fn test_detect_speech_low_freq() {
        let params = default_vad_config();
        let mut vad = SpectralVAD::new(params);
        let frame = generate_sine(100.0, 16000, 512);
        assert!(!vad.detect_speech(&frame, 16000));
    }

    #[test]
    fn test_detect_speech_speech_freq() {
        let params = default_vad_config();
        let mut vad = SpectralVAD::new(params);
        let frame = generate_sine(1000.0, 16000, 512);
        assert!(vad.detect_speech(&frame, 16000));
    }

    #[test]
    fn test_detect_speech_mixed_freq() {
        let mut params = default_vad_config();
        params.ratio_threshold = 1.5;
        let mut vad = SpectralVAD::new(params);
        let mut frame = vec![0i16; 512];
        let sine100 = generate_sine(100.0, 16000, 512);
        let sine1000 = generate_sine(1000.0, 16000, 512);
        for i in 0..512 {
            frame[i] = (sine100[i] as f32 * 0.2 + sine1000[i] as f32 * 0.8) as i16;
        }
        assert!(vad.detect_speech(&frame, 16000));
    }

    #[test]
    #[should_panic(expected = "Фрейм короче FFT размера")]
    fn test_panic_on_short_frame() {
        let params = default_vad_config();
        let mut vad = SpectralVAD::new(params);
        let frame = vec![0i16; 100];
        vad.detect_speech(&frame, 16000);
    }

    #[test]
    fn test_energy_threshold_boundary() {
        let mut params = default_vad_config();
        params.speech_energy_threshold = 1e9;
        let mut vad = SpectralVAD::new(params);
        let frame = generate_sine(1000.0, 16000, 512);
        assert!(!vad.detect_speech(&frame, 16000));
    }

    #[test]
    fn test_ratio_threshold_boundary() {
        let mut params = default_vad_config();
        params.ratio_threshold = 1e6;
        let mut vad = SpectralVAD::new(params);
        let frame = generate_sine(100.0, 16000, 512);
        assert!(!vad.detect_speech(&frame, 16000));
    }
}
