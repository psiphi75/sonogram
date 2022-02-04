/*
 * Copyright (C) Simon Werner, 2019.
 *
 * A Rust port of the original C++ code by Christian Briones, 2013.
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, see <http://www.gnu.org/licenses/>.
 */

use std::f32;
use std::sync::Arc;

use crate::{Spectrogram, WindowFn};
use rustfft::{num_complex::Complex, FftPlanner};

///
/// This contains all the initialised data.  This can then produce the spectrogram,
/// and if necessary, save it to the filesystem as a PNG image.
///
/// This `Spectrograph` is created by `SpecOptionsBuilder`.
///
/// # Example
///
/// ```Rust
///   let mut spectrograph = SpecOptionsBuilder::new(512, 128)
///     .load_data_from_file(&std::path::Path::new(wav_file))?
///     .build();
///  
///   spectrograph.compute(2048, 0.8);
///   spectrograph.save_as_png(&std::path::Path::new(png_file), false)?;
/// ```
///
pub struct SpecCompute {
  num_bins: usize,     // The num of fft bins in the spectrogram.
  data: Vec<f32>,      // The time domain data for the FFT.  Normalised to meet -1.0..1.0.
  window_fn: WindowFn, // The Window Function to apply to each fft window.
  step_size: usize, // The step size in the window function, must be less than the window function
  fft_fn: Arc<dyn rustfft::Fft<f32>>,
}

impl SpecCompute {
  /// Create a new Spectrograph from data.  
  ///
  /// **You probably want to use [SpecOptionsBuilder] instead.**
  pub fn new(num_bins: usize, step_size: usize, data: Vec<f32>, window_fn: WindowFn) -> Self {
    // Compute the FFT plan
    let mut planner = FftPlanner::<f32>::new();
    let fft_fn = planner.plan_fft_forward(num_bins);

    SpecCompute {
      num_bins,
      step_size,
      data,
      window_fn,
      fft_fn,
    }
  }

  pub fn set_data(&mut self, data: Vec<f32>) {
    self.data = data;
  }

  ///
  /// Do the discrete fourier transform to create the spectrogram.
  ///
  /// # Arguments
  ///
  ///  * `n_fft` - How many fourier transform frequency bins to use. Must be a
  ///                 power of 2.
  ///
  pub fn compute(&mut self) -> Spectrogram {
    let overlap = 1.0 - self.step_size as f32 / self.num_bins as f32;

    println!("Computing spectrogram...");
    println!("Bins: {}", self.num_bins);
    println!("Overlap: {}", overlap);
    println!("Step len: {}", self.step_size);
    println!("Number of samples: {}", self.data.len());

    self.run_fft()
  }

  fn run_fft(&mut self) -> Spectrogram {
    let width = (self.data.len() - self.num_bins) / self.step_size;

    let mut spectrogram = vec![vec![]];
    spectrogram.clear();
    spectrogram.reserve(width);

    let mut db_ref = f32::MIN;
    let mut p = 0;
    for _ in 0..width {
      let mut signal: Vec<Complex<f32>> = self.data[p..]
        .iter()
        .take(self.num_bins)
        .enumerate()
        .map(|(i, val)| val * (self.window_fn)(i, self.num_bins)) // Apply the window function
        .map(|val| Complex::new(val, 0.0))
        .collect();

      // Do the FFT, this will take the singal, and write back into to.
      // TODO: Slight performance improvement to use `process_with_scratch()`
      self.fft_fn.process(&mut signal);

      // Normalise the spectrogram
      let v_spec: Vec<f32> = signal
        .into_iter()
        .take(self.num_bins / 2)
        .rev()
        .map(|c_val| {
          let val = c_val.norm();
          db_ref = f32::max(db_ref, val);
          val
        })
        .collect();

      spectrogram.push(v_spec);

      p += self.step_size;
    }
    amplitude_to_db(&mut spectrogram, db_ref * db_ref);

    Spectrogram {
      data: spectrogram,
      width,
      height: self.num_bins / 2,
    }
  }
}

fn amplitude_to_db(spectrogram: &mut Vec<Vec<f32>>, amp_ref: f32) {
  let mut log_spec_max = f32::MIN;
  let offset = 10.0 * (f32::max(1e-10, amp_ref)).log10();

  for spec in spectrogram.iter_mut() {
    for i in 0..spec.len() {
      spec[i] = 10.0 * (f32::max(1e-10, spec[i] * spec[i])).log10() - offset;
      log_spec_max = f32::max(log_spec_max, spec[i]);
    }
  }

  for spec in spectrogram.iter_mut() {
    for i in 0..spec.len() {
      spec[i] = f32::max(spec[i], log_spec_max - 80.0);
    }
  }
}
