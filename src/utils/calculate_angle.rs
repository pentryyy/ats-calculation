/// Скорость звука в воздухе при 20°C (м/с)
const SPEED_OF_SOUND: f32 = 343.0;

/// Вычисляет угол прихода звука (в градусах) по двум сигналам.
///
/// # Аргументы
/// * `mic1`, `mic2` – векторы отсчётов (i16) с одинаковой частотой дискретизации
/// * `sample_rate` – частота дискретизации (Гц)
/// * `mic_distance` – расстояние между микрофонами (метры)
///
/// # Возвращает
/// Угол в градусах от -90 до +90 (отрицательный – звук слева, положительный – справа).
pub fn calculate_angle(mic1: &[i16], mic2: &[i16], sample_rate: u32, mic_distance: f32) -> f32 {
    let signal1: Vec<f32> = mic1.iter().map(|&x| x as f32).collect();
    let signal2: Vec<f32> = mic2.iter().map(|&x| x as f32).collect();

    let max_lag = (mic_distance * sample_rate as f32 / SPEED_OF_SOUND) as usize + 1;
    let n = signal1.len();
    let mut correlation = Vec::with_capacity(2 * max_lag + 1);

    let max_lag_isize = max_lag as isize;
    for lag in -max_lag_isize..=max_lag_isize {
        let mut sum = 0.0;
        let start1 = if lag < 0 { -lag } else { 0 };
        let start2 = if lag > 0 { lag } else { 0 };
        let len = n as isize - start1.max(start2);
        if len > 0 {
            for i in 0..(len as usize) {
                let idx1 = (start1 + i as isize) as usize;
                let idx2 = (start2 + i as isize) as usize;
                sum += signal1[idx1] * signal2[idx2];
            }
            correlation.push(sum);
        } else {
            correlation.push(0.0);
        }
    }

    let (max_index, _) = correlation
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .unwrap();
    let lag_samples = max_index as isize - max_lag_isize;

    let tau = lag_samples as f32 / sample_rate as f32;
    let ratio = (tau * SPEED_OF_SOUND / mic_distance).clamp(-1.0, 1.0);
    ratio.asin().to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn generate_sine_with_delay(
        freq_hz: f32,
        sample_rate: u32,
        num_samples: usize,
        delay_samples: isize,
    ) -> (Vec<i16>, Vec<i16>) {
        let mut mic1 = Vec::with_capacity(num_samples);
        let mut mic2 = Vec::with_capacity(num_samples);
        for n in 0..num_samples {
            let t1 = n as f32 / sample_rate as f32;
            let val1 = (2.0 * PI * freq_hz * t1).sin() * i16::MAX as f32;
            mic1.push(val1 as i16);

            let t2 = (n as f32 - delay_samples as f32) / sample_rate as f32;
            let val2 = (2.0 * PI * freq_hz * t2).sin() * i16::MAX as f32;
            mic2.push(val2 as i16);
        }
        (mic1, mic2)
    }

    fn generate_impulse_with_delay(
        num_samples: usize,
        delay_samples: isize,
    ) -> (Vec<i16>, Vec<i16>) {
        let mut mic1 = vec![0i16; num_samples];
        let mut mic2 = vec![0i16; num_samples];
        let delay = delay_samples.clamp(-(num_samples as isize - 1), num_samples as isize - 1);
        if delay >= 0 {
            mic1[0] = i16::MAX;
            mic2[delay as usize] = i16::MAX;
        } else {
            let shift = (-delay) as usize;
            mic1[shift] = i16::MAX;
            mic2[0] = i16::MAX;
        }
        (mic1, mic2)
    }

    #[test]
    fn test_zero_angle() {
        let sample_rate = 44100;
        let mic_distance = 0.2;
        let freq = 1000.0;
        let num_samples = 1024;
        let (mic1, mic2) = generate_sine_with_delay(freq, sample_rate, num_samples, 0);
        let angle = calculate_angle(&mic1, &mic2, sample_rate, mic_distance);
        assert!((angle - 0.0).abs() < 1e-5, "Expected 0°, got {}", angle);
    }

    #[test]
    fn test_positive_angle() {
        let sample_rate = 44100;
        let mic_distance = 0.2;
        let freq = 1000.0;
        let num_samples = 1024;
        let theta_deg = 30.0;
        let theta_rad = theta_deg * PI / 180.0;
        let delay_seconds = mic_distance * theta_rad.sin() / SPEED_OF_SOUND;
        let delay_samples = (delay_seconds * sample_rate as f32).round() as isize;
        let (mic1, mic2) = generate_sine_with_delay(freq, sample_rate, num_samples, delay_samples);
        let angle = calculate_angle(&mic1, &mic2, sample_rate, mic_distance);
        assert!(
            (angle - theta_deg).abs() < 1.0,
            "Expected around {}°, got {}",
            theta_deg,
            angle
        );
    }

    #[test]
    fn test_negative_angle() {
        let sample_rate = 44100;
        let mic_distance = 0.2;
        let freq = 1000.0;
        let num_samples = 1024;
        let theta_deg = -45.0;
        let theta_rad = theta_deg * PI / 180.0;
        let delay_seconds = mic_distance * theta_rad.sin() / SPEED_OF_SOUND;
        let delay_samples = (delay_seconds * sample_rate as f32).round() as isize;
        let (mic1, mic2) = generate_sine_with_delay(freq, sample_rate, num_samples, delay_samples);
        let angle = calculate_angle(&mic1, &mic2, sample_rate, mic_distance);
        assert!(
            (angle - theta_deg).abs() < 1.0,
            "Expected around {}°, got {}",
            theta_deg,
            angle
        );
    }

    #[test]
    fn test_max_angle_positive() {
        let sample_rate = 44100;
        let mic_distance = 0.2;
        let num_samples = 1024;
        let delay_seconds = mic_distance / SPEED_OF_SOUND;
        let delay_samples = (delay_seconds * sample_rate as f32).round() as isize;
        let (mic1, mic2) = generate_impulse_with_delay(num_samples, delay_samples);
        let angle = calculate_angle(&mic1, &mic2, sample_rate, mic_distance);
        assert!(
            (angle - 90.0).abs() < 1.0,
            "Expected around 90°, got {}",
            angle
        );
    }

    #[test]
    fn test_max_angle_negative() {
        let sample_rate = 44100;
        let mic_distance = 0.2;
        let num_samples = 1024;
        let delay_seconds = -mic_distance / SPEED_OF_SOUND;
        let delay_samples = (delay_seconds * sample_rate as f32).round() as isize;
        let (mic1, mic2) = generate_impulse_with_delay(num_samples, delay_samples);
        let angle = calculate_angle(&mic1, &mic2, sample_rate, mic_distance);
        assert!(
            (angle + 90.0).abs() < 1.0,
            "Expected around -90°, got {}",
            angle
        );
    }

    #[test]
    fn test_random_noise() {
        let sample_rate = 44100;
        let mic_distance = 0.2;
        let num_samples = 256;
        let mic1: Vec<i16> = (0..num_samples).map(|_| rand::random::<i16>()).collect();
        let mic2: Vec<i16> = (0..num_samples).map(|_| rand::random::<i16>()).collect();
        let angle = calculate_angle(&mic1, &mic2, sample_rate, mic_distance);
        assert!(
            (-90.0..=90.0).contains(&angle),
            "Angle out of range: {}",
            angle
        );
    }

    #[test]
    fn test_empty_slices() {
        let mic1: Vec<i16> = vec![];
        let mic2: Vec<i16> = vec![];
        let angle = calculate_angle(&mic1, &mic2, 44100, 0.2);
        assert!(angle.is_finite());
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn test_different_lengths() {
        let mic1: Vec<i16> = vec![1, 2, 3];
        let mic2: Vec<i16> = vec![1, 2];
        let _ = calculate_angle(&mic1, &mic2, 44100, 0.2);
    }

    #[test]
    fn test_zero_distance() {
        let sample_rate = 44100;
        let mic_distance = 0.0;
        let freq = 1000.0;
        let num_samples = 1024;
        let (mic1, mic2) = generate_sine_with_delay(freq, sample_rate, num_samples, 10);
        let angle = calculate_angle(&mic1, &mic2, sample_rate, mic_distance);
        assert!(
            (angle - 90.0).abs() < 1e-5 || (angle + 90.0).abs() < 1e-5,
            "Expected around +-90°, got {}",
            angle
        );
    }
}
