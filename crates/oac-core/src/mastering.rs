/// Masterizador de Áudio Padrão Audiolivro (EBU R128 / Audible Standard).
/// Alvo: -16.0 LUFS com Limiter True Peak em -1.0 dBTP (sem distorção digital).

pub const TARGET_LUFS: f32 = -16.0;
pub const MAX_TRUE_PEAK_I16: f32 = 29199.0; // -1.0 dBTP em relação a 32767.0
pub const MAX_GAIN_BOOST_DB: f32 = 12.0;

/// Biquad IIR Filter para K-Weighting (ITU-R BS.1770)
#[derive(Clone, Debug)]
struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Biquad {
    fn process(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// Coeficientes K-weighting para 24.000 Hz (frequência nativa do Kokoro-82M)
fn get_k_weighting_filters_24k() -> (Biquad, Biquad) {
    // Stage 1: High shelf (+4 dB acima de 1.5 kHz)
    let stage1 = Biquad {
        b0: 1.4968,
        b1: -2.0963,
        b2: 0.7712,
        a1: -1.3789,
        a2: 0.5506,
        x1: 0.0,
        x2: 0.0,
        y1: 0.0,
        y2: 0.0,
    };
    // Stage 2: High pass (RLB ~100 Hz cutoff)
    let stage2 = Biquad {
        b0: 1.0,
        b1: -2.0,
        b2: 1.0,
        a1: -1.9749,
        a2: 0.9754,
        x1: 0.0,
        x2: 0.0,
        y1: 0.0,
        y2: 0.0,
    };
    (stage1, stage2)
}

/// Mede o Loudness integrado aproximado em LUFS de um buffer de amostras i16 mono.
pub fn measure_loudness_lufs(samples: &[i16], sample_rate: u32) -> f32 {
    if samples.is_empty() {
        return -70.0;
    }

    let (mut f1, mut f2) = get_k_weighting_filters_24k();
    let _ = sample_rate;

    let mut sum_sq = 0.0f64;
    let mut count = 0usize;

    for &s in samples {
        // Normalizado para [-1.0, 1.0]
        let x = s as f64 / 32768.0;
        let y1 = f1.process(x);
        let y2 = f2.process(y1);
        sum_sq += y2 * y2;
        count += 1;
    }

    if count == 0 || sum_sq <= 1e-12 {
        return -70.0;
    }

    let mean_sq = sum_sq / (count as f64);
    let lufs = -0.691 + 10.0 * mean_sq.log10();
    lufs as f32
}

/// Normaliza um buffer de áudio em PCM 16-bit para -16.0 LUFS com teto em -1.0 dBTP.
pub fn normalize_to_ebu_r128(samples: &mut [i16], sample_rate: u32) {
    if samples.is_empty() {
        return;
    }

    let current_lufs = measure_loudness_lufs(samples, sample_rate);
    if current_lufs < -65.0 {
        // Silêncio quase absoluto: não amplificar ruído de fundo
        return;
    }

    let delta_db = (TARGET_LUFS - current_lufs).clamp(-18.0, MAX_GAIN_BOOST_DB);
    let linear_gain = 10.0f32.powf(delta_db / 20.0);

    // Aplicar ganho com limitador suave (soft-knee limiter) para impedir clipping acima de -1.0 dBTP
    let ceiling = MAX_TRUE_PEAK_I16;

    for s in samples.iter_mut() {
        let amplified = (*s as f32) * linear_gain;
        let abs_val = amplified.abs();

        if abs_val <= ceiling * 0.85 {
            *s = amplified.clamp(-32768.0, 32767.0) as i16;
        } else {
            // Curva suave de compressão (soft limiter com saturação suave)
            let sign = amplified.signum();
            let threshold = ceiling * 0.85;
            let excess = abs_val - threshold;
            let max_excess = ceiling - threshold;
            let compressed = threshold + max_excess * (excess / (excess + max_excess));
            *s = (sign * compressed).clamp(-ceiling, ceiling) as i16;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loudness_measurement_and_normalization() {
        // Gera tom senoidal com amplitude moderada
        let mut samples = Vec::new();
        for i in 0..24000 {
            let phase = 2.0 * std::f32::consts::PI * 440.0 * (i as f32 / 24000.0);
            let s = (phase.sin() * 8000.0) as i16;
            samples.push(s);
        }

        let lufs_before = measure_loudness_lufs(&samples, 24000);
        assert!(lufs_before < -10.0);

        normalize_to_ebu_r128(&mut samples, 24000);

        // Verifica que o pico máximo não ultrapassa o teto de -1.0 dBTP
        let max_peak = samples.iter().map(|&s| (s as f32).abs()).fold(0.0f32, f32::max);
        assert!(max_peak <= MAX_TRUE_PEAK_I16 + 1.0);
    }
}
