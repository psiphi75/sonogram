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
    /// * `height_orig` - the half the data length, i.e. the nyquist frequency.
    /// * `height_new` - The output grid/image height in cells/pixels.
    pub fn create(
        freq_scale: FrequencyScale,
        height_orig: usize,
        height_new: usize,
    ) -> Box<dyn FreqScalerTrait> {
        match freq_scale {
            FrequencyScale::Linear => {
                Box::new(LinearFreq::init(height_orig as f32, height_new as f32))
            }
            FrequencyScale::Log => Box::new(LogFreq::init(height_orig as f32, height_new as f32)),
        }
    }
}

pub trait FreqScalerTrait {
    /// Initialise the scaler object, can put cached values here.
    fn init(height_orig: f32, height: f32) -> Self
    where
        Self: Sized;

    /// The y->(f1,f2) scaler function
    fn scale(&self, y: usize) -> (f32, f32);
}

/// Scale the frequncy linearly.
pub struct LinearFreq {
    ratio: f32,
}

impl FreqScalerTrait for LinearFreq {
    /// Initialise the scaler.
    ///
    /// # Arguments
    ///
    /// * `height_orig` - the half the data length, i.e. the nyquist frequency.
    /// * `height_new` - The output grid/image height in cells/pixels.
    ///
    fn init(height_orig: f32, height_new: f32) -> Self {
        Self {
            ratio: height_orig / height_new,
        }
    }

    /// Scale the y axis value to match the y of the image.
    ///
    /// # Arguments
    ///
    /// * `height_orig` - the half the data length, i.e. the nyquist frequency.
    /// * `height_new` - The output grid/image height in cells/pixels.
    ///
    /// # Returns
    ///
    /// * A pair describing the lower bound and upper bound of the range.
    ///
    fn scale(&self, y: usize) -> (f32, f32) {
        let f1 = self.ratio * y as f32;
        let f2 = self.ratio * ((y + 1) as f32);
        (f1, f2)
    }
}

///
/// Scale the frequncy to a Log (base E) frequency scale.
///
pub struct LogFreq {
    height_orig: f32,
    height_new: f32,
    log_coef: f32,
}

impl FreqScalerTrait for LogFreq {
    ///
    /// Initialise the scaler.
    ///
    /// # Arguments
    ///
    /// * `height_orig` - the half the data length, i.e. the nyquist frequency.
    /// * `height_new` - The output grid/image height in cells/pixels.
    ///
    fn init(height_orig: f32, height_new: f32) -> Self {
        Self {
            height_orig,
            height_new,
            log_coef: 1.0 / (height_new + 1.0).ln() * height_orig,
        }
    }

    ///
    /// Scale the y axis value to match the y of the image.
    ///
    /// # Arguments
    ///
    /// * `height_orig` - the half the data length, i.e. the nyquist frequency.
    /// * `height_new` - The output grid/image height in cells/pixels.
    ///
    /// # Returns
    ///
    /// * A pair describing the lower bound and upper bound of the range
    ///
    fn scale(&self, y: usize) -> (f32, f32) {
        let f1 = self.height_orig - (self.log_coef * (self.height_new + 1.0 - y as f32).ln());
        let f2 = self.height_orig - (self.log_coef * (self.height_new + 1.0 - (y + 1) as f32).ln());
        (f1, f2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_scale_down() {
        let scale = LinearFreq::init(10.0, 5.0);

        let (h1, h2) = scale.scale(0);
        assert!((h1 - 0.0).abs() < 0.0001);
        assert!((h2 - 0.5).abs() < 0.0001);

        let (h1, h2) = scale.scale(6);
        assert!((h1 - 3.0).abs() < 0.0001);
        assert!((h2 - 3.5).abs() < 0.0001);
    }

    #[test]
    fn test_linear_scale_up() {
        let scale = LinearFreq::init(5.0, 10.0);

        let (h1, h2) = scale.scale(0);
        assert!((h1 - 0.0).abs() < 0.0001);
        assert!((h2 - 2.0).abs() < 0.0001);

        let (h1, h2) = scale.scale(6);
        assert!((h1 - 12.0).abs() < 0.0001);
        assert!((h2 - 14.0).abs() < 0.0001);
    }
}
