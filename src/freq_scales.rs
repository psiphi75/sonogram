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

//! Frequency Scaling for image data
//!
//! This module contains the [FreqScalerTrait] trait.  This trait allows
//! the frequency axis to be scaled using different methods.  For example
//! linear, log, mel, etc.

///
/// The Frequency scale to implement for the vertical axis.
///
#[derive(Clone, Copy)]
pub enum FrequencyScale {
  Linear,
  Log,
}

pub struct FreqScaler;

impl FreqScaler {
  ///
  /// Create an instance of [FreqScalerTrait] given the [FrequencyScale].
  ///
  /// # Arguments
  ///
  /// * `freq_scale` - The [FrequencyScale] to implement.
  /// * `nyquist_len` - the half the data length, i.e. the nyquist frequency.
  /// * `height` - The output grid/image height in cells/pixels.
  pub fn create(
    freq_scale: FrequencyScale,
    nyquist_len: f32,
    height: f32,
  ) -> Box<dyn FreqScalerTrait> {
    match freq_scale {
      FrequencyScale::Linear => Box::new(LinearFreq::init(nyquist_len, height)),
      FrequencyScale::Log => Box::new(LogFreq::init(nyquist_len, height)),
    }
  }
}

pub trait FreqScalerTrait {
  /// Initialise the scaler object, can put cached values here.
  fn init(nyquist_len: f32, height: f32) -> Self
  where
    Self: Sized;

  /// The y->(f1,f2) scaler function
  fn scale(&self, y: usize) -> (f32, f32);
}

///
/// Scale the frequncy to a Log (base E) frequency scale.
///
pub struct LogFreq {
  nyquist_len: f32,
  height: f32,
  log_coef: f32,
}

impl FreqScalerTrait for LogFreq {
  ///
  /// Initialise the scaler.
  ///
  /// # Arguments
  ///
  /// * `nyquist_len` - the half the data length, i.e. the nyquist frequency.
  /// * `height` - The output grid/image height in cells/pixels.
  ///
  fn init(nyquist_len: f32, height: f32) -> Self {
    Self {
      nyquist_len,
      height,
      log_coef: 1.0 / (height + 1.0).ln() * nyquist_len,
    }
  }

  ///
  /// Scale the y axis value to match the y of the image.
  ///
  /// # Arguments
  ///
  /// * `nyquist_len` - the half the data length, i.e. the nyquist frequency.
  /// * `height` - The output grid/image height in cells/pixels.
  ///
  /// # Returns
  ///
  /// * A pair describing the lower bound and upper bound of the range
  ///
  fn scale(&self, y: usize) -> (f32, f32) {
    let f1 = self.nyquist_len - (self.log_coef * (self.height + 1.0 - y as f32).ln());
    let f2 = self.nyquist_len - (self.log_coef * (self.height + 1.0 - (y + 1) as f32).ln());
    (f1, f2)
  }
}

/// Scale the frequncy linearly.
pub struct LinearFreq {
  nyquist_len: f32,
  height: f32,
}

impl FreqScalerTrait for LinearFreq {
  /// Initialise the scaler.
  ///
  /// # Arguments
  ///
  /// * `nyquist_len` - the half the data length, i.e. the nyquist frequency.
  /// * `height` - The output grid/image height in cells/pixels.
  ///
  fn init(nyquist_len: f32, height: f32) -> Self {
    Self {
      nyquist_len,
      height,
    }
  }

  /// Scale the y axis value to match the y of the image.
  ///
  /// # Arguments
  ///
  /// * `nyquist_len` - the half the data length, i.e. the nyquist frequency.
  /// * `height` - The output grid/image height in cells/pixels.
  ///
  /// # Returns
  ///
  /// * A pair describing the lower bound and upper bound of the range.
  ///
  fn scale(&self, y: usize) -> (f32, f32) {
    let f1 = (y as f32 / self.height) * self.nyquist_len;
    let f2 = ((y + 1) as f32 / self.height) * self.nyquist_len;
    (f1, f2)
  }
}
