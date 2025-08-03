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

use std::sync::Arc;
use std::{cmp::min, f32};

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
///   let mut spectrograph = SpecOptionsBuilder::new(2048)
///     .load_data_from_file(&std::path::Path::new(wav_file))?
///     .build();
///   
///   // Compute the spectrogram.  Need export it using `to_png()` or simlar.
///   spectrograph.compute();
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

    ///
    /// Update the sample data with a new set.  Note, none of the settings
    /// from the builder are applied, all the samples are used in their raw form.
    ///
    pub fn set_data(&mut self, data: Vec<f32>) {
        self.data = data;
    }

    ///
    /// Do the discrete fourier transform to create the spectrogram.
    ///
    /// # Arguments
    ///
    ///  * `n_fft` - How many fourier transform frequency bins to use. Must be a power of 2.
    ///
    pub fn compute(&mut self) -> Spectrogram {
        let width = (self.data.len() - self.num_bins) / self.step_size;
        let height = self.num_bins / 2;

        let mut spec = vec![0.0; self.num_bins * width];

        let mut p = 0; // Index to the beginning of the window

        // Once, Allocate buffers that will be used for computation
        let mut inplace_buf: Vec<Complex<f32>> = vec![Complex::new(0., 0.); self.num_bins];
        let mut scratch_buf: Vec<Complex<f32>> =
            vec![Complex::new(0., 0.); self.fft_fn.get_inplace_scratch_len()];

        // Create slices into the buffers backing the Vecs to be reused on each loop
        let inplace_slice = &mut inplace_buf[..];
        let scratch_slice = &mut scratch_buf[..];

        for w in 0..width {
            // Extract the next `num_bins` complex floats into the FFT inplace compute buffer
            self.data[p..]
                .iter()
                .take(self.num_bins)
                .enumerate()
                .map(|(i, val)| val * (self.window_fn)(i, self.num_bins)) // Apply the window function
                .map(|val| Complex::new(val, 0.0))
                .zip(inplace_slice.iter_mut())
                .for_each(|(c, v)| *v = c);

            // Call out to rustfft to actually compute the FFT
            // This will take the inplace_slice as input, use scratch_slice during computation, and write FFT back into inplace_slice
            let inplace = &mut inplace_slice[..min(self.num_bins, self.data.len() - p)];
            self.fft_fn.process_with_scratch(inplace, scratch_slice);

            // Normalize the spectrogram and write to the output
            inplace
                .iter()
                .take(height)
                .rev()
                .map(|c_val| c_val.norm())
                .zip(spec[w..].iter_mut().step_by(width))
                .for_each(|(a, b)| *b = a);

            p += self.step_size;
        }

        Spectrogram {
            spec,
            width,
            height,
        }
    }
}
