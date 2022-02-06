/*
 * Copyright (C) Simon Werner, 2022.
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
use std::path::Path;

use crate::errors::SonogramError;
use crate::window_fn;
use crate::SpecCompute;

type WindowFn = fn(usize, usize) -> f32;

///
/// A builder struct that will output a spectrogram creator when complete.
/// This builder will require the height and width of the final spectrogram,
/// at a minimum.  However you can load data from a .wav file, or directly
/// from a Vec<i16> memory object.
///
/// # Example
///
/// ```Rust
///   let mut spectrograph = SpecOptionsBuilder::new(512, 128)
///     .set_window_fn(utility::blackman_harris)
///     .load_data_from_file(&std::path::Path::new("test.wav"))?
///     .build();
/// ```
///
pub struct SpecOptionsBuilder {
    // Inputs
    data: Vec<f32>,                    // Our time-domain data (audio samples)
    sample_rate: u32,                  // The sample rate of the wav data
    channel: u16,                      // The audio channel
    scale_factor: Option<f32>,         // How much to scale the sample amplitude by
    do_normalise: bool,                // Normalise the samples to between -1.0...1.0
    downsample_divisor: Option<usize>, // Downsample the samples by a given amount

    // FFT info
    num_bins: usize,     // The number of FFT bins
    step_size: usize,    // How far to step between each window function
    window_fn: WindowFn, // The windowing function to use.
}

impl SpecOptionsBuilder {
    /// Create a new SpecOptionsBuilder.  The final height and width of
    /// the spectrogram must be supplied.  Before the `build` function
    /// can be called a `load_data_from_*` function needs to be called.
    ///
    /// # Arguments
    ///  
    ///  * `num_bins` - Number of bins in the discrete fourier transform (FFT)
    ///
    pub fn new(num_bins: usize) -> Self {
        SpecOptionsBuilder {
            data: vec![],
            sample_rate: 11025,
            channel: 1,
            scale_factor: None,
            do_normalise: false,
            downsample_divisor: None,
            num_bins,
            window_fn: window_fn::rectangular,
            step_size: num_bins,
        }
    }

    /// Load a .wav file to memory and use that file as the input.
    ///
    /// # Arguments
    ///
    ///  * `fname` - The path to the file.
    ///
    pub fn load_data_from_file(self, fname: &Path) -> Result<Self, SonogramError> {
        let mut reader = hound::WavReader::open(fname)?;

        // Can only handle 16 bit data
        // TODO: Add more data here
        if 16 != reader.spec().bits_per_sample {
            return Err(SonogramError::InvalidCodec);
        }

        if self.channel > reader.spec().channels {
            return Err(SonogramError::InvalidChannel);
        }

        let data: Vec<i16> = {
            let first_sample = self.channel as usize - 1;
            let step_size = reader.spec().channels as usize;
            let mut s = reader.samples();

            // TODO: replace this with .advanced_by in the future
            for _ in 0..first_sample {
                s.next();
            }

            s.step_by(step_size).map(|x| x.unwrap()).collect()
        };
        let sample_rate = reader.spec().sample_rate;

        Ok(self.load_data_from_memory(data, sample_rate))
    }

    /// Load data directly from memory - i16 version.
    ///
    /// # Arguments
    ///
    ///  * `data` - The raw wavform data that will be converted to a spectrogram.
    ///  * `sample_rate` - The sample rate, in Hz, of the data.
    ///
    pub fn load_data_from_memory(mut self, data: Vec<i16>, sample_rate: u32) -> Self {
        self.data = data.iter().map(|&x| x as f32 / (i16::MAX as f32)).collect();
        self.sample_rate = sample_rate;
        self
    }

    /// Load data directly from memory - f32 version.
    ///
    /// # Arguments
    ///
    ///  * `data` - The raw wavform data that will be converted to a spectrogram.
    ///             Samples must be in the range -1.0 to 1.0.
    ///  * `sample_rate` - The sample rate, in Hz, of the data.
    ///
    pub fn load_data_from_memory_f32(mut self, data: Vec<f32>, sample_rate: u32) -> Self {
        self.data = data;
        self.sample_rate = sample_rate;
        self
    }

    ///
    /// Down sample the data by the given divisor.  This is a cheap way of
    /// improving the performance of the FFT.
    ///
    /// # Arguments
    ///
    ///  * `divisor` - How much to reduce the data by.
    ///
    pub fn downsample(mut self, divisor: usize) -> Self {
        self.downsample_divisor = Some(divisor);
        self
    }

    ///
    /// Set the audio channel to use when importing a WAV file.
    /// By default this is 1.
    ///
    pub fn channel(mut self, channel: u16) -> Self {
        self.channel = channel;
        self
    }

    ///
    /// Normalise all the sample values to range from -1.0 to 1.0.
    ///
    pub fn normalise(mut self) -> Self {
        self.do_normalise = true;
        self
    }

    ///
    /// Scale the sample data by the given amount.
    ///
    pub fn scale(mut self, scale_factor: f32) -> Self {
        self.scale_factor = Some(scale_factor);
        self
    }

    /// A window function describes the type of window to use during the
    /// DFT (discrete fourier transform).  See
    /// (here)[https://en.wikipedia.org/wiki/Window_function] for more details.
    ///
    /// # Arguments
    ///
    ///  * `window` - The window function to be used.
    ///
    pub fn set_window_fn(mut self, window_fn: WindowFn) -> Self {
        self.window_fn = window_fn;
        self
    }

    ///
    /// This is the step size (as the number of samples) between each
    /// application of the window function.  A smaller step size may
    /// increase the smoothness of the sample, but take more time.  The default
    /// step size, if not set, is the same as the number of FFT bins.  This
    /// there is no overlap between windows and it most cases will suit your
    /// needs.
    ///
    pub fn set_step_size(mut self, step_size: usize) -> Self {
        self.step_size = step_size;
        self
    }

    ///
    /// The final method to be called.  This will create an instance of
    /// [Spectrograph].
    ///
    pub fn build(mut self) -> Result<SpecCompute, SonogramError> {
        if self.data.is_empty() {
            // SpecOptionsBuilder requires data to be loaded
            return Err(SonogramError::IncompleteData);
        }

        if self.channel == 0 {
            // The channel must be an integer 1 or greater
            return Err(SonogramError::InvalidChannel);
        }

        //
        // Do downsample
        //

        if let Some(divisor) = self.downsample_divisor {
            if divisor == 0 {
                return Err(SonogramError::InvalidDivisor);
            }

            if divisor > 1 {
                for (j, i) in (0..self.data.len() - divisor).step_by(divisor).enumerate() {
                    let sum: f32 = self.data[i..i + divisor].iter().fold(0.0, |mut sum, &val| {
                        sum += val;
                        sum
                    });
                    let avg = sum / (divisor as f32);

                    self.data[j] = avg;
                }
                self.data.resize(self.data.len() / divisor, 0.0);
                self.sample_rate /= divisor as u32;
            }
        }

        //
        // Normalise
        //

        if self.do_normalise {
            let max = self
                .data
                .iter()
                .reduce(|max, x| if x > max { x } else { max })
                .unwrap();

            let norm = 1.0 / max;
            for x in self.data.iter_mut() {
                *x *= norm;
            }
        }

        //
        // Apply the scale factor
        //

        if let Some(scale_factor) = self.scale_factor {
            for x in self.data.iter_mut() {
                *x *= scale_factor;
            }
        }

        Ok(SpecCompute::new(
            self.num_bins,
            self.step_size,
            self.data,
            self.window_fn,
        ))
    }
}
