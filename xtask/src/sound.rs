pub const SAMPLE_RATE: u32 = 48_000;

const Q15_ONE: i32 = 32_768;
// These cues deliberately favor a soft onset over the smallest possible payload.
// The previous 56 ms countdown pulse was mathematically continuous, but its fast
// attack and bright harmonic made the whole sound perceptually read as a click.
const COUNTDOWN_SAMPLES: usize = samples_for_milliseconds(120);
const ENTER_SAMPLES: usize = samples_for_milliseconds(400);
const LEAVE_SAMPLES: usize = samples_for_milliseconds(370);
const MOMENTARY_WINDOW_SAMPLES: usize = SAMPLE_RATE as usize / 10;
// Wwise recommends 100 ms Momentary Max normalization for short sound effects;
// its default normalization target is -23 dB.
// https://www.audiokinetic.com/library/2024.1.0_8598/?id=using_loudness_normalization_or_make_up_gain_to_adjust_volume&source=Help
const TARGET_MOMENTARY_RMS: u64 = 2_320;
// Leave generous headroom for transient and inter-sample peaks.
const PEAK_CEILING: u64 = 8_231;

const fn samples_for_milliseconds(milliseconds: usize) -> usize {
    SAMPLE_RATE as usize * milliseconds / 1_000
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cue {
    Countdown,
    GamingEnter,
    GamingLeave,
}

impl Cue {
    pub const ALL: [Self; 3] = [Self::Countdown, Self::GamingEnter, Self::GamingLeave];

    pub const fn file_name(self) -> &'static str {
        match self {
            Self::Countdown => "countdown.wav",
            Self::GamingEnter => "gaming-enter.wav",
            Self::GamingLeave => "gaming-leave.wav",
        }
    }

    const fn sample_count(self) -> usize {
        match self {
            Self::Countdown => COUNTDOWN_SAMPLES,
            Self::GamingEnter => ENTER_SAMPLES,
            Self::GamingLeave => LEAVE_SAMPLES,
        }
    }
}

pub fn wav_bytes(cue: Cue) -> Vec<u8> {
    let samples = pcm_samples(cue);
    encode_pcm_wav(&samples)
}

fn pcm_samples(cue: Cue) -> Vec<i16> {
    let raw = match cue {
        Cue::Countdown => synthesize_countdown(),
        Cue::GamingEnter => synthesize_gaming_enter(),
        Cue::GamingLeave => synthesize_gaming_leave(),
    };
    debug_assert_eq!(raw.len(), cue.sample_count());
    normalize_loudness(&raw)
}

fn synthesize_countdown() -> Vec<i32> {
    let mut carrier = Oscillator::default();
    let mut sub = Oscillator::default();

    (0..COUNTDOWN_SAMPLES)
        .map(|index| {
            let frequency = glide_millihz(920_000, 700_000, index, COUNTDOWN_SAMPLES);
            let phase = carrier.next_phase(frequency);
            let sub_voice = sub.next(frequency / 2);
            let tonal = mix(&[(sine_q15(phase), 29_491), (sub_voice, 3_277)]);
            let envelope = padded_edge_envelope(
                index,
                COUNTDOWN_SAMPLES,
                samples_for_milliseconds(8),
                samples_for_milliseconds(28),
                samples_for_milliseconds(54),
                samples_for_milliseconds(14),
            );
            mul_q15(tonal, envelope)
        })
        .collect()
}

fn synthesize_gaming_enter() -> Vec<i32> {
    let mut carrier = Oscillator::default();
    let mut fifth = Oscillator::default();
    let mut sub = Oscillator::default();

    (0..ENTER_SAMPLES)
        .map(|index| {
            let frequency = glide_millihz(240_000, 780_000, index, ENTER_SAMPLES);
            let phase = carrier.next_phase(frequency);
            let tonal = mix(&[
                (sine_q15(phase), 24_904),
                (fifth.next(frequency * 3 / 2), 5_243),
                (sub.next(frequency / 2), 2_621),
            ]);
            let envelope = padded_edge_envelope(
                index,
                ENTER_SAMPLES,
                samples_for_milliseconds(10),
                samples_for_milliseconds(44),
                samples_for_milliseconds(90),
                samples_for_milliseconds(20),
            );
            mul_q15(tonal, envelope)
        })
        .collect()
}

fn synthesize_gaming_leave() -> Vec<i32> {
    let mut carrier = Oscillator::default();
    let mut fifth = Oscillator::default();
    let mut sub = Oscillator::default();

    (0..LEAVE_SAMPLES)
        .map(|index| {
            let frequency = glide_millihz(820_000, 250_000, index, LEAVE_SAMPLES);
            let phase = carrier.next_phase(frequency);
            let tonal = mix(&[
                (sine_q15(phase), 24_904),
                (fifth.next(frequency * 3 / 2), 4_915),
                (sub.next(frequency / 2), 2_949),
            ]);
            let envelope = padded_edge_envelope(
                index,
                LEAVE_SAMPLES,
                samples_for_milliseconds(10),
                samples_for_milliseconds(42),
                samples_for_milliseconds(100),
                samples_for_milliseconds(22),
            );
            mul_q15(tonal, envelope)
        })
        .collect()
}

fn normalize_loudness(samples: &[i32]) -> Vec<i16> {
    let taper = samples
        .iter()
        .enumerate()
        .map(|(index, _)| {
            edge_envelope(
                index,
                samples.len(),
                samples_for_milliseconds(12),
                samples_for_milliseconds(32),
            )
        })
        .collect::<Vec<_>>();
    let tapered = samples
        .iter()
        .zip(&taper)
        .map(|(sample, taper)| mul_q15(*sample, *taper))
        .collect::<Vec<_>>();
    // Remove any sub-LSB finite-window DC bias without sacrificing the exact
    // zero-valued endpoints: shape the correction by the same C2 taper.
    let taper_sum = taper.iter().map(|value| i64::from(*value)).sum::<i64>();
    let correction = tapered.iter().map(|sample| i64::from(*sample)).sum::<i64>()
        * i64::from(Q15_ONE)
        / taper_sum.max(1);
    let centered = tapered
        .iter()
        .zip(&taper)
        .map(|(sample, taper)| *sample - mul_q15(correction as i32, *taper))
        .collect::<Vec<_>>();
    let peak = centered
        .iter()
        .map(|sample| i64::from(*sample).unsigned_abs())
        .max()
        .unwrap_or(1)
        .max(1);
    let momentary_rms = maximum_window_rms(&centered, MOMENTARY_WINDOW_SAMPLES).max(1);
    let (gain_numerator, gain_denominator) =
        if TARGET_MOMENTARY_RMS * peak <= PEAK_CEILING * momentary_rms {
            (TARGET_MOMENTARY_RMS, momentary_rms)
        } else {
            (PEAK_CEILING, peak)
        };

    centered
        .iter()
        .map(|sample| {
            let product = i128::from(*sample) * i128::from(gain_numerator);
            let half = i128::from(gain_denominator / 2);
            let rounded = if product >= 0 {
                (product + half) / i128::from(gain_denominator)
            } else {
                (product - half) / i128::from(gain_denominator)
            };
            rounded.clamp(i128::from(i16::MIN), i128::from(i16::MAX)) as i16
        })
        .collect()
}

fn maximum_window_rms(samples: &[i32], maximum_window: usize) -> u64 {
    let window = samples.len().min(maximum_window).max(1);
    let mut sum = samples[..window]
        .iter()
        .map(|sample| i128::from(*sample) * i128::from(*sample))
        .sum::<i128>();
    let mut maximum_sum = sum;

    for index in window..samples.len() {
        sum += i128::from(samples[index]) * i128::from(samples[index]);
        sum -= i128::from(samples[index - window]) * i128::from(samples[index - window]);
        maximum_sum = maximum_sum.max(sum);
    }

    integer_sqrt((maximum_sum / window as i128) as u128) as u64
}

fn integer_sqrt(value: u128) -> u128 {
    if value < 2 {
        return value;
    }

    // Start above the square root. Using ceil(log2 / 2) is too small when
    // log2 is even and makes Newton's method return before converging.
    let mut estimate = 1_u128 << (value.ilog2() / 2 + 1);
    loop {
        let next = (estimate + value / estimate) / 2;
        if next >= estimate {
            return estimate;
        }
        estimate = next;
    }
}

fn encode_pcm_wav(samples: &[i16]) -> Vec<u8> {
    let data_size = std::mem::size_of_val(samples) as u32;
    let mut bytes = Vec::with_capacity(44 + data_size as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

#[derive(Default)]
struct Oscillator {
    phase: u32,
}

impl Oscillator {
    fn next(&mut self, frequency_millihz: u32) -> i32 {
        sine_q15(self.next_phase(frequency_millihz))
    }

    fn next_phase(&mut self, frequency_millihz: u32) -> u32 {
        let phase = self.phase;
        let increment = ((u64::from(frequency_millihz) * (1_u64 << 32))
            / (u64::from(SAMPLE_RATE) * 1_000)) as u32;
        self.phase = self.phase.wrapping_add(increment);
        phase
    }
}

fn sine_q15(phase: u32) -> i32 {
    const QUARTER_TURN: i64 = 1_i64 << 30;
    const QUADRANT_MASK: u32 = (1_u32 << 30) - 1;
    const CORDIC_ANGLES: [i64; 20] = [
        536_870_912,
        316_933_406,
        167_458_907,
        85_004_756,
        42_667_331,
        21_354_465,
        10_679_838,
        5_340_245,
        2_670_163,
        1_335_087,
        667_544,
        333_772,
        166_886,
        83_443,
        41_722,
        20_861,
        10_430,
        5_215,
        2_608,
        1_304,
    ];

    let quadrant = phase >> 30;
    let offset = i64::from(phase & QUADRANT_MASK);
    if offset == 0 {
        return match quadrant {
            0 | 2 => 0,
            1 => 32_767,
            3 => -32_767,
            _ => unreachable!(),
        };
    }

    let (mut angle, sign) = match quadrant {
        0 => (offset, 1),
        1 => (QUARTER_TURN - offset, 1),
        2 => (offset, -1),
        3 => (QUARTER_TURN - offset, -1),
        _ => unreachable!(),
    };
    const AMPLITUDE_FRACTION_BITS: u32 = 16;
    let mut x = 1_304_025_951_i64;
    let mut y = 0_i64;

    for (shift, step) in CORDIC_ANGLES.into_iter().enumerate() {
        let (next_x, next_y) = if angle >= 0 {
            (x - (y >> shift), y + (x >> shift))
        } else {
            (x + (y >> shift), y - (x >> shift))
        };
        x = next_x;
        y = next_y;
        angle += if angle >= 0 { -step } else { step };
    }

    let signed = sign * y;
    let rounded = if signed >= 0 {
        (signed + (1_i64 << (AMPLITUDE_FRACTION_BITS - 1))) >> AMPLITUDE_FRACTION_BITS
    } else {
        (signed - (1_i64 << (AMPLITUDE_FRACTION_BITS - 1))) >> AMPLITUDE_FRACTION_BITS
    };
    rounded.clamp(-32_767, 32_767) as i32
}

fn glide_millihz(start: u32, end: u32, index: usize, sample_count: usize) -> u32 {
    let progress = smootherstep_q15(ratio_q15(index, sample_count.saturating_sub(1)));
    let delta = i64::from(end) - i64::from(start);
    (i64::from(start) + delta * i64::from(progress) / i64::from(Q15_ONE)) as u32
}

fn edge_envelope(index: usize, sample_count: usize, attack: usize, release: usize) -> i32 {
    let attack = smootherstep_q15(ratio_q15(index, attack));
    let release = smootherstep_q15(ratio_q15(sample_count - 1 - index, release));
    mul_q15(attack, release)
}

fn padded_edge_envelope(
    index: usize,
    sample_count: usize,
    leading_silence: usize,
    attack: usize,
    release: usize,
    trailing_silence: usize,
) -> i32 {
    let active_sample_count = sample_count.saturating_sub(leading_silence + trailing_silence);
    if index < leading_silence || index >= leading_silence + active_sample_count {
        return 0;
    }

    edge_envelope(
        index - leading_silence,
        active_sample_count,
        attack,
        release,
    )
}

fn ratio_q15(numerator: usize, denominator: usize) -> i32 {
    if denominator == 0 {
        return Q15_ONE;
    }
    ((numerator.min(denominator) as i64 * i64::from(Q15_ONE)) / denominator as i64) as i32
}

fn smootherstep_q15(value: i32) -> i32 {
    let value = value.clamp(0, Q15_ONE);
    let square = mul_q15(value, value);
    let cube = mul_q15(square, value);
    mul_q15(cube, 10 * Q15_ONE - 15 * value + 6 * square)
}

fn mul_q15(left: i32, right: i32) -> i32 {
    ((i64::from(left) * i64::from(right)) / i64::from(Q15_ONE)) as i32
}

fn mix(voices: &[(i32, i32)]) -> i32 {
    voices
        .iter()
        .map(|(sample, gain)| mul_q15(*sample, *gain))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_wavs_are_valid_mono_pcm_and_tiny() {
        let mut total_size = 0;

        for cue in Cue::ALL {
            let bytes = wav_bytes(cue);
            let sample_count = cue.sample_count();
            total_size += bytes.len();

            assert_eq!(&bytes[0..4], b"RIFF");
            assert_eq!(&bytes[8..16], b"WAVEfmt ");
            assert_eq!(read_u32(&bytes, 4), bytes.len() as u32 - 8);
            assert_eq!(read_u32(&bytes, 16), 16);
            assert_eq!(read_u16(&bytes, 20), 1);
            assert_eq!(read_u16(&bytes, 22), 1);
            assert_eq!(read_u32(&bytes, 24), SAMPLE_RATE);
            assert_eq!(read_u32(&bytes, 28), SAMPLE_RATE * 2);
            assert_eq!(read_u16(&bytes, 32), 2);
            assert_eq!(read_u16(&bytes, 34), 16);
            assert_eq!(&bytes[36..40], b"data");
            assert_eq!(read_u32(&bytes, 40), (sample_count * 2) as u32);
            assert_eq!(bytes.len(), 44 + sample_count * 2);
        }

        assert!(total_size <= 90 * 1_024);
    }

    #[test]
    fn generated_signals_are_click_safe_and_healthy() {
        for cue in Cue::ALL {
            let samples = pcm_samples(cue);
            let samples_i32 = samples
                .iter()
                .map(|sample| i32::from(*sample))
                .collect::<Vec<_>>();
            let peak = samples
                .iter()
                .map(|sample| i32::from(*sample).unsigned_abs())
                .max()
                .unwrap();
            let momentary_rms = maximum_window_rms(&samples_i32, MOMENTARY_WINDOW_SAMPLES);
            let ten_millisecond_rms = maximum_window_rms(&samples_i32, SAMPLE_RATE as usize / 100);
            let sum = samples_i32
                .iter()
                .map(|sample| i64::from(*sample))
                .sum::<i64>();
            let maximum_jump = samples
                .windows(2)
                .map(|pair| (i32::from(pair[1]) - i32::from(pair[0])).unsigned_abs())
                .max()
                .unwrap();
            let mut second_differences = samples
                .windows(3)
                .map(|window| {
                    (i32::from(window[2]) - 2 * i32::from(window[1]) + i32::from(window[0]))
                        .unsigned_abs()
                })
                .collect::<Vec<_>>();
            second_differences.sort_unstable();
            let percentile_999 = second_differences[second_differences.len() * 999 / 1_000];
            let maximum_second_difference = *second_differences.last().unwrap();
            let second_difference_rms = maximum_window_rms(
                &samples_i32
                    .windows(3)
                    .map(|window| window[2] - 2 * window[1] + window[0])
                    .collect::<Vec<_>>(),
                samples_i32.len(),
            );
            let full_rms = maximum_window_rms(&samples_i32, samples_i32.len());

            assert_eq!(samples[0], 0);
            assert_eq!(samples[samples.len() - 1], 0);
            assert!(peak <= PEAK_CEILING as u32 + 1);
            assert!((1_800..=TARGET_MOMENTARY_RMS + 1).contains(&momentary_rms));
            assert!(ten_millisecond_rms <= 5_500);
            assert!(sum.unsigned_abs() * 2 <= samples.len() as u64);
            assert!(maximum_jump < 2_000);
            assert!(maximum_second_difference <= percentile_999 * 6 / 5 + 8);
            assert!(second_difference_rms * 25 <= full_rms);
            assert!(
                samples[..samples_for_milliseconds(5)]
                    .iter()
                    .all(|sample| *sample == 0)
            );
            assert!(
                samples[samples.len() - samples_for_milliseconds(8)..]
                    .iter()
                    .all(|sample| *sample == 0)
            );
        }
    }

    #[test]
    fn synthesis_is_deterministic_and_each_cue_is_distinct() {
        for cue in Cue::ALL {
            assert_eq!(wav_bytes(cue), wav_bytes(cue));
        }
        assert_ne!(wav_bytes(Cue::Countdown), wav_bytes(Cue::GamingEnter));
        assert_ne!(wav_bytes(Cue::GamingEnter), wav_bytes(Cue::GamingLeave));
    }

    #[test]
    fn glide_directions_match_the_mode_transitions() {
        assert_eq!(glide_millihz(240_000, 780_000, 0, 100), 240_000);
        assert_eq!(glide_millihz(240_000, 780_000, 99, 100), 780_000);
        assert_eq!(glide_millihz(820_000, 250_000, 0, 100), 820_000);
        assert_eq!(glide_millihz(820_000, 250_000, 99, 100), 250_000);
    }

    #[test]
    fn oscillator_error_stays_below_minus_seventy_decibels() {
        let maximum_error = (0..4_096_u64)
            .map(|index| {
                let phase = ((index << 32) / 4_096) as u32;
                let angle = index as f64 * std::f64::consts::TAU / 4_096.0;
                let ideal = (angle.sin() * 32_767.0).round() as i32;
                (sine_q15(phase) - ideal).unsigned_abs()
            })
            .max()
            .unwrap();

        assert!(maximum_error <= 4, "maximum sine error was {maximum_error}");
    }

    #[test]
    fn integer_square_root_is_exact_or_floored() {
        for value in [0, 1, 2, 3, 4, 8, 9, 15, 16, 5_840_000] {
            let root = integer_sqrt(value);
            assert!(root * root <= value);
            assert!((root + 1) * (root + 1) > value);
        }
    }

    fn read_u16(bytes: &[u8], offset: usize) -> u16 {
        u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
    }

    fn read_u32(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }
}
