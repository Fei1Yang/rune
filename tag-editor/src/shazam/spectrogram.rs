use rustfft::{num_complex::Complex, FftPlanner};
use std::{cmp, fmt};

// Time offsets for peak detection
const TIME_OFFSETS: [i32; 14] = [
    -53, -45, 165, 172, 179, 186, 193, 200, 214, 221, 228, 235, 242, 249,
];

// Neighbor offsets for frequency peak detection
const NEIGHBORS: [i32; 8] = [-10, -7, -4, -3, 1, 2, 5, 8];

#[derive(Debug)]
pub struct FrequencyPeak {
    pub pass: i32,
    pub magnitude: i32,
    pub bin: i32,
}

#[derive(Debug)]
pub struct Signature {
    pub sample_rate: i32,
    pub num_samples: i32,
    pub peaks_by_band: [Vec<FrequencyPeak>; 5],
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Signature:")?;
        writeln!(f, "= Sample Rate: {} Hz", self.sample_rate)?;
        writeln!(f, "= Total Samples: {}", self.num_samples)?;

        // Display the number of peaks in each frequency band
        for (band_index, peaks) in self.peaks_by_band.iter().enumerate() {
            writeln!(f, "= Band {}: {} peaks", band_index, peaks.len())?;
        }

        Ok(())
    }
}

// Define a new type for the array
#[derive(Clone)]
struct F64Array1025([f64; 1025]);

impl Default for F64Array1025 {
    fn default() -> Self {
        F64Array1025([0.0; 1025])
    }
}

struct Ring<T: Clone + Default> {
    buf: Vec<T>,
    index: usize,
}

impl<T: Clone + Default> Ring<T> {
    fn new(size: usize) -> Self {
        Ring {
            buf: vec![T::default(); size],
            index: 0,
        }
    }

    fn mod_index(&self, i: i32) -> usize {
        let len = self.buf.len() as i32;
        let mut idx = i;
        while idx < 0 {
            idx += len;
        }
        (idx as usize) % self.buf.len()
    }

    fn at(&self, i: i32) -> &T {
        let idx = self.mod_index(self.index as i32 + i);
        &self.buf[idx]
    }

    fn append(&mut self, values: &[T]) {
        for x in values {
            self.buf[self.index] = x.clone();
            self.index = (self.index + 1) % self.buf.len();
        }
    }

    fn slice(&self, offset: i32, dest: &mut [T]) {
        let mut curr_offset = self.mod_index(offset + self.index as i32);
        for item in dest.iter_mut() {
            *item = self.buf[curr_offset].clone();
            curr_offset = (curr_offset + 1) % self.buf.len();
        }
    }
}

pub fn compute_signature(sample_rate: i32, samples: &[f64]) -> Signature {
    let max_neighbor = |spread_outputs: &Ring<F64Array1025>, i: usize| {
        let mut neighbor = 0.0f64;
        for &off in NEIGHBORS.iter() {
            let idx = i as isize + off as isize;
            if (0..1025).contains(&idx) {
                neighbor = neighbor.max(spread_outputs.at(-49).0[idx as usize]);
            }
        }
        for &off in TIME_OFFSETS.iter() {
            let idx = i as isize - 1;
            if (0..1025).contains(&idx) {
                neighbor = neighbor.max(spread_outputs.at(off).0[idx as usize]);
            }
        }
        neighbor
    };

    let normalize_peak = |x: f64| (x.max(1.0 / 64.0)).ln() * 1477.3 + 6144.0;

    let peak_band = |bin: i32| -> Option<usize> {
        let hz = (bin * sample_rate) / (2 * 1024 * 64);
        match hz {
            hz if (250..520).contains(&hz) => Some(0),
            hz if (520..1450).contains(&hz) => Some(1),
            hz if (1450..3500).contains(&hz) => Some(2),
            hz if (3500..=5500).contains(&hz) => Some(3),
            _ => None,
        }
    };

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(2048);

    let mut samples_ring = Ring::<f64>::new(2048);
    let mut fft_outputs = Ring::<F64Array1025>::new(256);
    let mut spread_outputs = Ring::<F64Array1025>::new(256);
    let mut peaks_by_band: [Vec<FrequencyPeak>; 5] = Default::default();

    // Hanning window multipliers
    let hanning: Vec<f64> = (0..2048)
        .map(|i| 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / 2048.0).cos()))
        .collect();

    for i in 0..(samples.len() - 128) / 128 {
        let start = i * 128;
        let end = cmp::min(start + 128, samples.len());
        samples_ring.append(&samples[start..end]);

        let mut reordered_samples = vec![Complex::new(0.0f64, 0.0f64); 2048];
        let mut temp = vec![0.0f64; 2048];
        samples_ring.slice(0, &mut temp);

        // Apply Hanning window and prepare complex input
        for (j, &sample) in temp.iter().enumerate() {
            reordered_samples[j] = Complex::new((sample * 1024.0 * 64.0).round() * hanning[j], 0.0);
        }

        // Perform FFT
        fft.process(&mut reordered_samples);

        let mut outputs = [0.0f64; 1025];
        for j in 0..1025 {
            outputs[j] = (reordered_samples[j].re * reordered_samples[j].re
                + reordered_samples[j].im * reordered_samples[j].im)
                / ((1 << 17) as f64).max(0.0000000001);
        }
        fft_outputs.append(&[F64Array1025(outputs)]);

        // Spread peaks in frequency domain
        let mut spread = outputs;
        for j in 0..outputs.len() - 2 {
            spread[j] = outputs[j..=j + 2].iter().fold(0.0f64, |a, &b| a.max(b));
        }
        spread_outputs.append(&[F64Array1025(spread)]);

        // Spread in time domain
        for &off in &[-2, -4, -7] {
            let prev = spread_outputs.at(off);
            for (output, &prev_val) in outputs.iter_mut().zip(prev.0.iter()) {
                *output = output.max(prev_val);
            }
        }

        // Skip until we have enough samples
        if i < 45 {
            continue;
        }

        // Recognize peaks
        let fft_output = fft_outputs.at(-46);
        for bin in 10..1015 {
            if fft_output.0[bin] <= max_neighbor(&spread_outputs, bin) {
                continue;
            }

            let before = normalize_peak(fft_output.0[bin - 1]);
            let peak = normalize_peak(fft_output.0[bin]);
            let after = normalize_peak(fft_output.0[bin + 1]);
            let variation = ((32.0 * (after - before)) / (2.0 * peak - after - before)) as i32;
            let peak_bin = bin as i32 * 64 + variation;

            if let Some(band) = peak_band(peak_bin) {
                peaks_by_band[band].push(FrequencyPeak {
                    pass: i as i32 - 45,
                    magnitude: peak as i32,
                    bin: peak_bin,
                });
            }
        }
    }

    Signature {
        sample_rate,
        num_samples: samples.len() as i32,
        peaks_by_band,
    }
}
