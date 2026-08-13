use rustfft::{FftPlanner, num_complex::Complex};
use std::sync::Arc;

pub struct SpectralVAD {
    fft: Arc<dyn rustfft::Fft<f32>>,
    buffer: Vec<Complex<f32>>,
}

impl SpectralVAD {
    pub fn new(fft_size: usize) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(fft_size);
        Self {
            fft,
            buffer: vec![Complex::new(0.0, 0.0); fft_size],
        }
    }

    pub fn detect_speech(&mut self, frame: &[i16], sample_rate: u32) -> bool {
        let n = self.buffer.len();
        assert!(frame.len() >= n, "Фрейм короче FFT размера");

        for i in 0..n {
            let window =
                0.54 - 0.46 * (2.0 * std::f32::consts::PI * i as f32 / (n - 1) as f32).cos();
            self.buffer[i] = Complex::new(frame[i] as f32 * window, 0.0);
        }

        self.fft.process(&mut self.buffer);

        let low_bin = (200 * n as u32 / sample_rate) as usize;
        let speech_bin_start = (300 * n as u32 / sample_rate) as usize;
        let speech_bin_end = (3000 * n as u32 / sample_rate) as usize;

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

        speech_energy > 0.1 && (speech_energy / (low_energy + 0.001)) > 1.8
    }
}
