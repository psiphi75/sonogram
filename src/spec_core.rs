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

use std::sync::{Arc};
use std::{cmp::min, f32};

use crate::{Spectrogram, WindowFn};
use rustfft::{num_complex::Complex, FftPlanner};

#[cfg(feature = "rayon")]
use rayon::prelude::*;

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
    fft_fn: Arc<dyn rustfft::Fft<f32>>, // An FFT function that can be used to compute the FFT.

    #[cfg(feature = "rayon")]
    gen_fft_fn: Arc<dyn Sync + Send + Fn() -> Arc<dyn rustfft::Fft<f32>>>, // An FFT generator for each thread
}

impl SpecCompute {
    /// Create a new Spectrograph from data.  
    ///
    /// **You probably want to use [SpecOptionsBuilder] instead.**
    pub fn new(num_bins: usize, step_size: usize, data: Vec<f32>, window_fn: WindowFn) -> Self {
        // Compute the FFT plan generator and a single FFT plan, if supporting parallel FFTs.
        #[cfg(feature = "rayon")]
        {
            let gen_fft_fn = Arc::new(move || {
                let mut planner = FftPlanner::new();
                planner.plan_fft_forward(num_bins)
            });
            let fft_fn = gen_fft_fn();
            SpecCompute {
                num_bins,
                step_size,
                data,
                window_fn,
                fft_fn,
                gen_fft_fn,
            }
        }

        // Compute a single FFT plan, if not supporting parallel FFTs.
        #[cfg(not(feature = "rayon"))]
        {
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
    ///  * `n_fft` - How many fourier transform frequency bins to use. Must be a
    ///                 power of 2.
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

    ///
    /// Do the discrete fourier transform to create the spectrogram.
    /// 
    /// This function will use rayon to parallelize the FFT computation.
    /// It may create more FFT plans than there are threads, but will reuse them 
    /// if called multiple times.
    ///
    ///
    /// # Arguments
    /// 
    ///  * `self` -  `self` is immutable so that the FFT plans can be shared between threads.
    ///               Since the FFT plans are not thread safe, they are wrapped in a Mutex. 
    ///
    ///  * `data` -  The time domain data to compute the spectrogram from.
    ///              `data` is not preprocessed other than windowing and 
    ///               casting to complex.
    ///
    #[cfg(feature = "rayon")]
    pub fn par_compute(&self, data: &[f32]) -> Spectrogram {
        use std::cell::RefCell;

        let width = (data.len() - self.num_bins) / self.step_size;
        let height = self.num_bins / 2;

        // Compute the spectrogram in parallel steps via rayon
        let spec_cols: Vec<_> = (0..width)
            .into_par_iter()
            .map(
                |w| {
                    // Grab the FFT for this thread
                    thread_local! {
                        pub static FFT_FN: RefCell<Option<Arc<dyn rustfft::Fft<f32>>>> = RefCell::new(None);
                        pub static INPLACE_BUF: RefCell<Option<Vec<Complex<f32>>>> = RefCell::new(None);
                        pub static SCRATCH_BUF: RefCell<Option<Vec<Complex<f32>>>> = RefCell::new(None);
                    }

                    // Index to the beginning of the window
                    let window_index = w * self.step_size;

                    // All the computation on the column happens in this thread where we have exclusive access to the memory
                    let spec_col = FFT_FN.with(|fft_fn| {
                        let fft_fn = fft_fn.borrow_mut().get_or_insert_with(|| (self.gen_fft_fn)()).clone();
                        INPLACE_BUF.with(|inplace_buf| {
                            let mut inplace_buf_ref = inplace_buf.borrow_mut();
                            let inplace_buf = inplace_buf_ref.get_or_insert_with(|| vec![Complex::new(0., 0.); self.num_bins]);
                            SCRATCH_BUF.with(|scratch_buf| {
                                let mut scratch_buf_ref = scratch_buf.borrow_mut();
                                let scratch_buf = scratch_buf_ref.get_or_insert_with(|| vec![Complex::new(0., 0.); fft_fn.get_inplace_scratch_len()]);

                                // Extract the next `num_bins` complex floats into the FFT inplace compute buffer
                                data[(w * self.step_size)..]
                                .iter()
                                .take(self.num_bins)
                                .enumerate()
                                .map(|(i, val)| Complex::new(val * (self.window_fn)(i, self.num_bins), 0.))
                                .zip(inplace_buf.iter_mut())
                                .for_each(|(c, v)| *v = c);
            
                                // Create slices into the buffers backing the Vecs to be reused on each loop
                                let inplace_slice = inplace_buf.as_mut_slice();
                                let scratch_slice = scratch_buf.as_mut_slice();
            
                                // Call out to rustfft to actually compute the FFT
                                // This will take the inplace_slice as input, use scratch_slice during computation, and write FFT back into inplace_slice
                                let inplace =
                                    &mut inplace_slice[..min(self.num_bins, data.len() - window_index)];
                                fft_fn.process_with_scratch(inplace, scratch_slice);
            
                                // Normalize the spectrogram and write to the output
                                inplace
                                    .iter()
                                    .take(height)
                                    .rev()
                                    .map(|c_val| c_val.norm())
                                    .collect::<Vec<_>>()
                            })
                        })
                    });

                    spec_col
                },
            )
            .collect();


        // Transpose the columns into row major order
        let spec = (0..height)
            .into_par_iter()
            .flat_map_iter(|i| spec_cols.iter().map(move |row| row[i]))
            .collect::<Vec<_>>();

        Spectrogram {
            spec,
            width,
            height,
        }
    }
}
