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

use crate::colour_gradient::ColourGradient;
use crate::errors::SonogramError;
use crate::utility;
use crate::ColourTheme;
use crate::Spectrograph;

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
  width: usize,             // The width of the output
  height: usize,            // The height of the output
  sample_rate: u32,         // The sample rate of the wav data
  data: Vec<f32>,           // Our raw wav data
  channel: u16,             // The audio channel
  window: WindowFn,         // The windowing function to use.
  verbose: bool,            // Do we print out stats and things
  gradient: ColourGradient, // User defined colour gradient
}

impl SpecOptionsBuilder {
  /// Create a new SpecOptionsBuilder.  The final height and width of
  /// the spectrogram must be supplied.  Before the `build` function
  /// can be called a `load_data_from_*` function needs to be called.
  ///
  /// # Arguments
  ///  
  ///  * `width` - The final width of the spectrogram.
  ///  * `height` - The final height of the spectrogram.
  ///
  pub fn new(width: usize, height: usize) -> Self {
    SpecOptionsBuilder {
      width,
      height,
      sample_rate: 8000,
      data: vec![],
      channel: 1,
      window: utility::rectangular,
      verbose: false,
      gradient: ColourGradient::create(ColourTheme::Default),
    }
  }

  /// A window function describes the type of window to use during the
  /// DFT (discrete fourier transform).  See
  /// (here)[https://en.wikipedia.org/wiki/Window_function] for more details.
  ///
  /// # Arguments
  ///
  ///  * `window` - The window function to be used.
  ///
  pub fn set_window_fn(&mut self, window_fn: WindowFn) -> &mut Self {
    self.window = window_fn;
    self
  }

  /// Load a .wav file to memory and use that file as the input.
  ///
  /// # Arguments
  ///
  ///  * `fname` - The path to the file.
  ///
  pub fn load_data_from_file(&mut self, fname: &Path) -> Result<&mut Self, SonogramError> {
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
  pub fn load_data_from_memory(&mut self, data: Vec<i16>, sample_rate: u32) -> &mut Self {
    self.data = data.iter().map(|&x| x as f32 / (i16::MAX as f32)).collect();
    self.sample_rate = sample_rate;
    self
  }

  /// Load data directly from memory - f32 version.
  ///
  /// # Arguments
  ///
  ///  * `data` - The raw wavform data that will be converted to a spectrogram. Samples must be
  ///             in the range -1.0 to 1.0.
  ///  * `sample_rate` - The sample rate, in Hz, of the data.
  ///
  pub fn load_data_from_memory_f32(&mut self, data: Vec<f32>, sample_rate: u32) -> &mut Self {
    self.data = data;
    self.sample_rate = sample_rate;
    self
  }

  ///
  /// Down sample the data by the given divisor.  This is a cheap way of improving the
  /// performance of the FFT.
  ///
  /// # Arguments
  ///
  ///  * `divisor` - How much to reduce the data by.
  ///
  pub fn downsample(&mut self, divisor: usize) -> Result<&mut Self, SonogramError> {
    if divisor == 0 {
      return Err(SonogramError::InvalidDivisor);
    }
    if divisor == 1 {
      return Ok(self);
    }
    if self.data.is_empty() {
      return Err(SonogramError::IncompleteData);
    }

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

    Ok(self)
  }

  pub fn channel(&mut self, channel: u16) -> Result<&mut Self, SonogramError> {
    if channel == 0 {
      // The channel must be an integer 1 or greater
      return Err(SonogramError::InvalidChannel);
    }
    self.channel = channel;
    Ok(self)
  }

  pub fn set_verbose(&mut self) -> &mut Self {
    self.verbose = true;
    self
  }

  pub fn set_gradient(&mut self, gradient: ColourGradient) -> &mut Self {
    self.gradient = gradient;
    self
  }

  pub fn normalise(&mut self) -> &mut Self {
    let max = self
      .data
      .iter()
      .reduce(|max, x| if x > max { x } else { max })
      .unwrap();
    let norm = 1.0 / max;
    self.data = self.data.iter().map(|x| x * norm).collect();
    self
  }

  pub fn scale(&mut self, scale_factor: f32) -> Result<&mut Self, SonogramError> {
    if self.data.is_empty() {
      // Need to load the data before calling scale
      return Err(SonogramError::IncompleteData);
    }

    // Don't need to scale 1.0
    if scale_factor == 1.0 {
      return Ok(self);
    }

    self.data = self.data.iter().map(|x| x * scale_factor).collect();
    Ok(self)
  }

  /// Last method to be called.  This will calculate the colour gradients and
  /// generate an instance of [Spectrograph].
  pub fn build(&self) -> Result<Spectrograph, SonogramError> {
    if self.data.is_empty() {
      // SpecOptionsBuilder requires data to be loaded
      return Err(SonogramError::IncompleteData);
    }

    let audio_length_sec = self.data.len() as u32 / self.sample_rate;
    if self.verbose {
      println!("Length (s): {}", audio_length_sec);
    }

    Ok(Spectrograph::new(
      self.width,
      self.height,
      self.data.clone(), // TODO: There's probably more efficient ways of doing this
      self.window,
      self.gradient.clone(),
      self.verbose,
    ))
  }
}
