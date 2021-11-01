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

use num::complex::Complex;
use std::f32;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use hound;
use png::HasParameters; // To use encoder.set()

use crate::colour_gradient::{ColourGradient, RGBAColour};
use crate::errors::SonogramError;
use crate::utility;

type Spectrogram = Vec<Vec<Complex<f32>>>;
type WindowFn = fn(u32, u32) -> f32;

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
  width: u32,       // The width of the output
  height: u32,      // The height of the output
  sample_rate: u32, // The sample rate of the wav data
  data: Vec<f32>,   // Our raw wav data
  channel: u16,     // The audio channel
  window: WindowFn, // The windowing function to use.
  greyscale: bool,  // Is the output in greyscale
  verbose: bool,    // Do we print out stats and things
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
  pub fn new(width: u32, height: u32) -> Self {
    SpecOptionsBuilder {
      width,
      height,
      sample_rate: 8000,
      data: vec![],
      channel: 1,
      window: utility::rectangular,
      greyscale: false,
      verbose: false,
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
  pub fn set_window_fn(&mut self, window: fn(u32, u32) -> f32) -> &mut Self {
    self.window = window;
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
      panic!("Currently we can only handle 16 bits per sample in the audio source.");
    }

    if self.channel > reader.spec().channels {
      panic!("Channel set to {}, but the audio on has {} channel(s)", self.channel, reader.spec().channels);
    }

    let data: Vec<i16> = {
      let first_sample = self.channel as usize - 1;
      let step_size = reader.spec().channels as usize;
      let mut s = reader.samples();

      // TODO: replace this with .advanced_by in the future
      for _ in 0..first_sample {
        s.next();
      }

      s.step_by(step_size)
        .map(|x| x.unwrap())
        .collect()
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
    self.data = data.iter().map(|&x| x as f32).collect();
    self.sample_rate = sample_rate;
    self
  }

  /// Load data directly from memory - f32 version.
  ///
  /// # Arguments
  ///
  ///  * `data` - The raw wavform data that will be converted to a spectrogram.
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
  pub fn downsample(&mut self, divisor: usize) -> &mut Self {
    if divisor == 0 {
      panic!("The divisor is too small");
    }
    if divisor == 1 {
      return self;
    }
    if self.data.is_empty() {
      panic!("Need to load the data before calling downsample");
    }

    let mut j = 0;
    for i in (0..self.data.len() - divisor).step_by(divisor) {
      let sum: f32 = self.data[i..i + divisor].iter().fold(0.0, |mut sum, &val| {
        sum += val;
        sum
      });
      let avg = sum / (divisor as f32);

      self.data[j] = avg;
      j += 1;
    }
    self.data.resize(self.data.len() / divisor, 0.0);
    self.sample_rate /= divisor as u32;

    self
  }

  pub fn channel(&mut self, channel: u16) -> &mut Self {
    if channel == 0 {
      panic!("The channel must be an integer 1 or greater");
    }
    self.channel = channel;
    self
  }

  pub fn set_verbose(&mut self) -> &mut Self {
    self.verbose = true;
    self
  }

  pub fn set_greyscale(&mut self) -> &mut Self {
    self.greyscale = true;
    self
  }

  pub fn scale(&mut self, scale_factor: f32) -> &mut Self {
    if self.data.is_empty() {
      panic!("Need to load the data before calling scale");
    }

    // Don't need to scale 1.0
    if scale_factor == 1.0 {
      return self;
    }

    for i in 0..self.data.len() {
      self.data[i] = self.data[i] * scale_factor;
    }

    self
  }

  /// Last method to be called.  This will calculate the colour gradients and
  /// generate an instance of `Spectrograph`.
  ///
  pub fn build(&self) -> Spectrograph {
    if self.data.is_empty() {
      panic!("SpecOptionsBuilder requires data to be loaded")
    }

    let audio_length_sec = self.data.len() as u32 / self.sample_rate;
    if self.verbose {
      println!("Length (s): {}", audio_length_sec);
    }

    let mut gradient = ColourGradient::new();

    if self.greyscale {
      gradient.add_colour(RGBAColour::new(0, 0, 0, 0));
      gradient.add_colour(RGBAColour::new(255, 255, 255, 0));
    } else {
      // Colour for our plot
      // Black
      gradient.add_colour(RGBAColour::new(0, 0, 0, 0));
      // Purple
      gradient.add_colour(RGBAColour::new(55, 0, 110, 0));
      // Blue
      gradient.add_colour(RGBAColour::new(0, 0, 180, 0));
      // Cyan
      gradient.add_colour(RGBAColour::new(0, 255, 255, 0));
      // Green
      gradient.add_colour(RGBAColour::new(0, 255, 0, 0));
      // Green Yellow
      // Yellow
      gradient.add_colour(RGBAColour::new(255, 255, 0, 0));
      // Orange
      gradient.add_colour(RGBAColour::new(230, 160, 0, 0));
      // Red
      gradient.add_colour(RGBAColour::new(255, 0, 0, 0));
    }

    Spectrograph {
      width: self.width,
      height: self.height,
      data: self.data.clone(), // TODO: There's probably more efficient ways of doing this
      window: self.window,
      spectrogram: vec![vec![]],
      gradient,
      verbose: self.verbose,
    }
  }
}

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
pub struct Spectrograph {
  width: u32,
  height: u32,
  data: Vec<f32>,
  window: WindowFn,
  spectrogram: Spectrogram,
  gradient: ColourGradient,
  verbose: bool,
}

impl Spectrograph {
  pub fn omega(&self, p: f32, q: f32) -> Complex<f32> {
    let trig_arg = 2.0 * f32::consts::PI * q / p;

    // VVV Comment out this line to use the cache VVVV
    Complex::new(f32::cos(trig_arg), f32::sin(trig_arg))
  }

  ///
  /// Do the discrete fourier transform to create the spectrogram.
  ///
  /// # Arguments
  ///
  ///  * `chunk_len` - How long fourier transform window should be.
  ///  * `overlap` - By how much (of a chunk) the fourier transform
  ///                windows overlap.  It must be value between 0.0
  ///                and 1.0.
  pub fn compute(&mut self, chunk_len: usize, overlap: f32) {
    assert!(0.0 <= overlap && overlap < 1.0);
    let step = (chunk_len as f32 * (1.0 - overlap)) as usize;

    if self.verbose {
      // Print out computation info
      println!("Computing spectrogram...");
      println!(
        "Chunk: {} Overlap: {}",
        chunk_len,
        overlap * chunk_len as f32
      );
      println!("Step len: {}", step);
      println!("Data len: {}", self.data.len());
    }

    // Pad the data
    let mut new_len = 0;
    while new_len + chunk_len < self.data.len() {
      new_len += step;
    }
    if new_len != self.data.len() {
      new_len += chunk_len;
      let padding = &mut vec![0.0; new_len - self.data.len()];
      self.data.append(padding);
    }

    self.chunkify(chunk_len, step);
  }

  ///
  /// Save the calculated spectrogram as a PNG image.
  ///
  /// # Arguments
  ///
  ///  * `fname` - The path to the PNG to save to the filesystem.
  ///  * `log_freq` - Apply the log function to the frequency scale.
  ///
  pub fn save_as_png(&mut self, fname: &Path, log_freq: bool) -> Result<(), std::io::Error> {
    let data_len = self.spectrogram[0].len();
    // Only the data below 1/2 of the sampling rate (nyquist frequency)
    // is useful
    let multiplier = 0.5;
    let img_len_used = data_len as f32 * multiplier;

    let log_coef = 1.0 / (self.height as f32 + 1.0).log(f32::consts::E) * img_len_used;

    let file = File::create(fname)?;
    let w = &mut BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, self.width, self.height);
    encoder.set(png::ColorType::RGBA).set(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;

    let mut img: Vec<u8> = vec![];

    for y in (0..self.height).rev() {
      for x in 0..self.width {
        let freq = if log_freq {
          img_len_used - (log_coef * (self.height as f32 + 1.0 - y as f32).log(f32::consts::E))
        } else {
          let ratio = y as f32 / self.height as f32;
          ratio * img_len_used
        };

        let colour = self.get_colour(self.spectrogram[x as usize][freq as usize], 15.0);
        img.extend(colour.to_vec());
      }
    }

    writer.write_image_data(&img)?; // Save

    Ok(())
  }

  ///
  /// Save the calculated spectrogram as a CSV file.
  ///
  /// # Arguments
  ///
  ///  * `fname` - The path to the CSV to save to the filesystem.
  ///  * `log_freq` - Apply the log function to the frequency scale.
  ///
  pub fn save_as_csv(&mut self, fname: &Path, log_freq: bool) -> Result<(), std::io::Error> {
    let data_len = self.spectrogram[0].len();
    // Only the data below 1/2 of the sampling rate (nyquist frequency)
    // is useful
    let multiplier = 0.5;
    let img_len_used = data_len as f32 * multiplier;

    let log_coef = 1.0 / (self.height as f32 + 1.0).log(f32::consts::E) * img_len_used;

    let mut writer = csv::Writer::from_path(fname)?;

    // Create the CSV header
    let mut csv_record: Vec<String> = (0..self.width).into_iter().map(|x| x.to_string()).collect();
    writer.write_record(&csv_record)?;

    for y in (0..self.height).rev() {
      for x in 0..self.width {
        let freq = if log_freq {
          img_len_used - (log_coef * (self.height as f32 + 1.0 - y as f32).log(f32::consts::E))
        } else {
          let ratio = y as f32 / self.height as f32;
          ratio * img_len_used
        };

        let value = self.get_real(self.spectrogram[x as usize][freq as usize]);
        csv_record[x as usize] = value.to_string();
      }
      writer.write_record(&csv_record)?;
    }

    writer.flush()?; // Save

    Ok(())
  }

  ///
  /// Create the spectrogram in memory.
  ///
  /// # Arguments
  ///
  ///  * `log_freq` - Apply the log function to the frequency scale.
  ///
  pub fn create_in_memory(&mut self, log_freq: bool) -> Vec<f32> {
    let data_len = self.spectrogram[0].len();
    // Only the data below 1/2 of the sampling rate (nyquist frequency)
    // is useful
    let multiplier = 0.5;
    let img_len_used = data_len as f32 * multiplier;

    let log_coef = 1.0 / (self.height as f32 + 1.0).log(f32::consts::E) * img_len_used;

    let mut result = Vec::with_capacity((self.height * self.width) as usize);

    for y in (0..self.height).rev() {
      for x in 0..self.width {
        let freq = if log_freq {
          img_len_used - (log_coef * (self.height as f32 + 1.0 - y as f32).log(f32::consts::E))
        } else {
          let ratio = y as f32 / self.height as f32;
          ratio * img_len_used
        };

        let value = self.get_real(self.spectrogram[x as usize][freq as usize]);
        result.push(value)
      }
    }

    result
  }

  fn get_real(&mut self, c: Complex<f32>) -> f32 {
    0.5 * (c.norm_sqr() + 1.0).log10()
  }

  fn get_colour(&mut self, c: Complex<f32>, threshold: f32) -> RGBAColour {
    let value = self.get_real(c);
    self.gradient.set_max(threshold);
    self.gradient.get_colour(value).clone()
  }

  fn get_number_of_chunks(&mut self, chunk_len: usize, step: usize) -> usize {
    let mut i = 0;
    let mut chunks = 0;
    while i + chunk_len <= self.data.len() {
      i += step;
      chunks += 1;
    }
    if i == self.data.len() {
      chunks -= 1;
    }
    chunks
  }

  fn chunkify(&mut self, chunk_len: usize, step: usize) {
    self.spectrogram.clear();
    self.spectrogram.reserve(self.width as usize);

    let num_chunks = self.get_number_of_chunks(chunk_len, step) as f32;
    if self.verbose {
      println!("Number of Chunks: {}", num_chunks);
    }

    let chunk_width_ratio = num_chunks / self.width as f32;

    let mut j = 0.0;

    while j < num_chunks {
      let p = j as usize * step;
      let mut signal: Vec<Complex<f32>> = self.data[p..]
        .iter()
        .take(chunk_len)
        .map(|d| Complex::new(f32::from(*d), 0.0))
        .collect();

      self.transform(&mut signal);
      self.spectrogram.push(signal);

      j += chunk_width_ratio;
    }
  }

  // TODO: Cache calculations of omega
  fn transform(&mut self, signal: &mut Vec<Complex<f32>>) {
    let min_len = signal.len();
    let power = pad_to_power2(signal, min_len);

    if power == 0 {
      return;
    }

    let mut transformed = vec![Complex::new(0f32, 0f32); signal.len()];
    // Apply window function and sort by bit-reversed index
    for i in 0..signal.len() {
      transformed[utility::reverse_bits(i, power)] =
        signal[i] * (self.window)(i as u32, signal.len() as u32);
    }

    let mut n = 2;
    while n <= transformed.len() {
      // Iterate over length-n segments
      let mut i = 0;
      while i <= transformed.len() - n {
        // Combine each half of the segment
        for m in i..(i + n / 2) {
          let term1 = transformed[m];
          let term2 = self.omega(n as f32, -(m as f32)) * transformed[m + n / 2];

          transformed[m] = term1 + term2;
          transformed[m + n / 2] = term1 - term2;
        }
        i += n;
      }
      n *= 2;
    }
    // Copy the data back into signal
    signal.clear();
    signal.extend(transformed.into_iter());
  }
}

fn pad_to_power2(signal: &mut Vec<Complex<f32>>, min_len: usize) -> usize {
  let mut power = 1;
  let mut new_len = 2;

  while new_len < min_len {
    new_len *= 2;
    power += 1;
  }
  pad(signal, new_len);
  let padding = &mut vec![Complex::new(0.0, 0.0); new_len - signal.len()];
  signal.append(padding);

  power
}

fn pad(signal: &mut Vec<Complex<f32>>, new_len: usize) {
  if new_len > signal.len() {
    signal.resize_with(new_len, || Complex::new(0.0, 0.0));
  }
}
