/*
 * Copyright (C) Simon Werner, 2022
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

extern crate csv;
#[cfg(feature = "png")]
extern crate png;

mod builder;
mod colour_gradient;
mod errors;
mod freq_scales;
mod spec_core;
mod window_fn;

pub use builder::SpecOptionsBuilder;
pub use colour_gradient::{ColourGradient, ColourTheme, RGBAColour};
pub use errors::SonogramError;
pub use freq_scales::{FreqScaler, FrequencyScale};
pub use spec_core::SpecCompute;
pub use window_fn::*;

#[cfg(feature = "png")]
use std::fs::File;
#[cfg(feature = "png")]
use std::io::BufWriter;
use std::path::Path;

use resize::Pixel::GrayF32;
use resize::Type::Lanczos3;
use rgb::FromSlice;

#[cfg(feature = "png")]
use png::HasParameters; // To use encoder.set()

#[cfg(feature = "rayon")]
use rayon::prelude::*;

pub struct Spectrogram {
    spec: Vec<f32>,
    width: usize,
    height: usize,
}

impl Spectrogram {
    ///
    /// Save the calculated spectrogram as a PNG image.
    ///
    /// # Arguments
    ///
    ///  * `fname` - The path to the PNG to save to the filesystem.
    ///  * `freq_scale` - The type of frequency scale to use for the spectrogram.
    ///  * `gradient` - The colour gradient to use for the spectrogram.
    ///  * `w_img` - The output image width.
    ///  * `h_img` - The output image height.
    ///
    #[cfg(feature = "png")]
    pub fn to_png(
        &mut self,
        fname: &Path,
        freq_scale: FrequencyScale,
        gradient: &mut ColourGradient,
        w_img: usize,
        h_img: usize,
    ) -> Result<(), std::io::Error> {
        let buf = self.to_buffer(freq_scale, w_img, h_img);

        let mut img: Vec<u8> = vec![0u8; w_img * h_img * 4];
        self.buf_to_img(&buf, &mut img, gradient);

        let file = File::create(fname)?;
        let w = &mut BufWriter::new(file);
        let mut encoder = png::Encoder::new(w, w_img as u32, h_img as u32);
        encoder.set(png::ColorType::RGBA).set(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&img)?; // Save

        Ok(())
    }

    ///
    /// Create the spectrogram in memory as a PNG.
    ///
    /// # Arguments
    ///
    ///  * `freq_scale` - The type of frequency scale to use for the spectrogram.
    ///  * `gradient` - The colour gradient to use for the spectrogram.
    ///  * `w_img` - The output image width.
    ///  * `h_img` - The output image height.
    ///
    #[cfg(feature = "png")]
    pub fn to_png_in_memory(
        &mut self,
        freq_scale: FrequencyScale,
        gradient: &mut ColourGradient,
        w_img: usize,
        h_img: usize,
    ) -> Result<Vec<u8>, std::io::Error> {
        let buf = self.to_buffer(freq_scale, w_img, h_img);

        let mut img: Vec<u8> = vec![0u8; w_img * h_img * 4];
        self.buf_to_img(&buf, &mut img, gradient);

        let mut pngbuf: Vec<u8> = Vec::new();
        let mut encoder = png::Encoder::new(&mut pngbuf, w_img as u32, h_img as u32);
        encoder.set(png::ColorType::RGBA).set(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&img)?;

        // The png writer needs to be explicitly dropped
        drop(writer);
        Ok(pngbuf)
    }

    ///
    /// Create the spectrogram in memory as raw RGBA format.
    ///
    /// # Arguments
    ///
    ///  * `freq_scale` - The type of frequency scale to use for the spectrogram.
    ///  * `gradient` - The colour gradient to use for the spectrogram.
    ///  * `w_img` - The output image width.
    ///  * `h_img` - The output image height.
    ///
    pub fn to_rgba_in_memory(
        &mut self,
        freq_scale: FrequencyScale,
        gradient: &mut ColourGradient,
        w_img: usize,
        h_img: usize,
    ) -> Vec<u8> {
        let buf = self.to_buffer(freq_scale, w_img, h_img);

        let mut img: Vec<u8> = vec![0u8; w_img * h_img * 4];
        self.buf_to_img(&buf, &mut img, gradient);

        img
    }

    /// Convenience function to convert the the buffer to an image
    fn buf_to_img(&self, buf: &[f32], img: &mut [u8], gradient: &mut ColourGradient) {
        let (min, max) = get_min_max(buf);
        gradient.set_min(min);
        gradient.set_max(max);

        // For each pixel, compute the RGBAColour, then assign each byte to output img
        buf.iter()
            .map(|val| gradient.get_colour(*val))
            .flat_map(|c| [c.r, c.g, c.b, c.a].into_iter())
            .zip(img.iter_mut())
            .for_each(|(val_rgba, img_rgba)| *img_rgba = val_rgba);
    }

    ///
    /// Save the calculated spectrogram as a CSV file.
    ///
    /// # Arguments
    ///
    ///  * `fname` - The path to the CSV to save to the filesystem.
    ///  * `freq_scale` - The type of frequency scale to use for the spectrogram.
    ///  * `cols` - The number of columns.
    ///  * `rows` - The number of rows.
    ///
    pub fn to_csv(
        &mut self,
        fname: &Path,
        freq_scale: FrequencyScale,
        cols: usize,
        rows: usize,
    ) -> Result<(), std::io::Error> {
        let result = self.to_buffer(freq_scale, cols, rows);

        let mut writer = csv::Writer::from_path(fname)?;

        // Create the CSV header
        let mut csv_record: Vec<String> = (0..cols).into_iter().map(|x| x.to_string()).collect();
        writer.write_record(&csv_record)?;

        let mut i = 0;
        for _ in 0..rows {
            for c_rec in csv_record.iter_mut().take(cols) {
                let val = result[i];
                i += 1;
                *c_rec = val.to_string();
            }
            writer.write_record(&csv_record)?;
        }

        writer.flush()?; // Save

        Ok(())
    }

    ///
    /// Map the spectrogram to the output buffer.  Essentially scales the
    /// frequency to map to the vertical axis (y-axis) of the output and
    /// scale the x-axis to match the output.  It will also convert the
    /// spectrogram to dB.
    ///
    /// # Arguments
    ///
    ///  * `freq_scale` - The type of frequency scale to use for the spectrogram.
    ///  * `img_width` - The output image width.
    ///  * `img_height` - The output image height.
    ///
    pub fn to_buffer(
        &self,
        freq_scale: FrequencyScale,
        img_width: usize,
        img_height: usize,
    ) -> Vec<f32> {
        let mut buf = Vec::with_capacity(self.height * self.width);

        // Apply the log scale if required
        match freq_scale {
            FrequencyScale::Log => {
                let scaler = FreqScaler::create(freq_scale, self.height, self.height);
                let mut vert_slice = vec![0.0; self.height];
                for h in 0..self.height {
                    let (f1, f2) = scaler.scale(h);
                    let (h1, mut h2) = (f1.floor() as usize, f2.ceil() as usize);
                    if h2 >= self.height {
                        h2 = self.height - 1;
                    }
                    for w in 0..self.width {
                        for (hh, val) in vert_slice.iter_mut().enumerate().take(h2).skip(h1) {
                            *val = self.spec[(hh * self.width) + w];
                        }
                        let value = integrate(f1, f2, &vert_slice);
                        buf.push(value);
                    }
                }
            }
            FrequencyScale::Linear => {
                buf.clone_from(&self.spec);
            }
        }

        // Convert the buffer to dB
        to_db(&mut buf);

        resize(&buf, self.width, self.height, img_width, img_height)
    }

    ///
    /// Get the minimum and maximum values from the current spectrogram.
    ///
    pub fn get_min_max(&self) -> (f32, f32) {
        get_min_max(&self.spec)
    }

    ///
    /// Get an iterator over the spectrogram data by row index.
    ///
    pub fn row_iter<'a>(&'a self, row_idx: usize) -> impl Iterator<Item = &'a f32> + 'a {
        self.spec
            .chunks_exact(self.width)
            .skip(row_idx)
            .flatten()
            .take(self.width)
    }
}

pub fn get_min_max(data: &[f32]) -> (f32, f32) {
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    for val in data {
        min = f32::min(*val, min);
        max = f32::max(*val, max);
    }
    (min, max)
}

#[cfg(feature = "rayon")]
fn to_db(buf: &mut [f32]) {
    let ref_db = buf
        .par_chunks(1_000)
        .fold(
            || f32::MIN,
            |acc, chunk| {
                let v = chunk.iter().fold(f32::MIN, |acc, &v| f32::max(acc, v));
                if acc > v {
                    acc
                } else {
                    v
                }
            },
        )
        .reduce(|| f32::MIN, |acc, v| f32::max(acc, v));

    let amp_ref = ref_db * ref_db;
    let offset = 10.0 * (f32::max(1e-10, amp_ref)).log10();
    let log_spec_max = buf
        .par_iter_mut()
        .map(|val| {
            *val = 10.0 * (f32::max(1e-10, *val * *val)).log10() - offset;
            *val
        })
        .fold(|| f32::MIN, |acc, v| f32::max(acc, v))
        .reduce(|| f32::MIN, |acc, v| f32::max(acc, v));
    let log_spec_max = log_spec_max - 80.0; // Why 80?  I don't know

    buf.par_chunks_mut(1_000).for_each(|chunk| {
        for val in chunk.iter_mut() {
            *val = f32::max(*val, log_spec_max);
        }
    });
}

#[cfg(not(feature = "rayon"))]
fn to_db(buf: &mut [f32]) {
    let mut ref_db = f32::MIN;
    buf.iter().for_each(|v| ref_db = f32::max(ref_db, *v));

    let amp_ref = ref_db * ref_db;
    let offset = 10.0 * (f32::max(1e-10, amp_ref)).log10();
    let mut log_spec_max = f32::MIN;

    for val in buf.iter_mut() {
        *val = 10.0 * (f32::max(1e-10, *val * *val)).log10() - offset;
        log_spec_max = f32::max(log_spec_max, *val);
    }

    for val in buf.iter_mut() {
        *val = f32::max(*val, log_spec_max - 80.0);
    }
}

///
/// Resize the image buffer
///
fn resize(buf: &[f32], w_in: usize, h_in: usize, w_out: usize, h_out: usize) -> Vec<f32> {
    // Resize the buffer to match the user requirements
    if let Ok(mut resizer) = resize::new(w_in, h_in, w_out, h_out, GrayF32, Lanczos3) {
        let mut resized_buf = vec![0.0; w_out * h_out];
        let result = resizer.resize(buf.as_gray(), resized_buf.as_gray_mut());
        if result.is_ok() {
            return resized_buf;
        }
    }

    // If this happens there resize return an Err
    vec![]
}

///
/// Integrate `spec` from `x1` to `x2`, where `x1` and `x2` are
/// floating point indicies where we take the fractional component into
/// account as well.
///
/// Integration is uses simple linear interpolation.
///
/// # Arguments
///
/// * `x1` - The fractional index that points to `spec`.
/// * `x2` - The fractional index that points to `spec`.
/// * `spec` - The values that require integration.
///
/// # Returns
///
/// The result of the integration.
///
fn integrate(x1: f32, x2: f32, spec: &[f32]) -> f32 {
    let mut i_x1 = x1.floor() as usize;
    let i_x2 = (x2 - 0.000001).floor() as usize;

    // Calculate the ratio from
    let area = |y, frac| y * frac;

    if i_x1 >= i_x2 {
        // Sub-cell integration
        area(spec[i_x1], x2 - x1)
    } else {
        // Need to integrate from x1 to x2 over multiple indicies.
        let mut result = area(spec[i_x1], (i_x1 + 1) as f32 - x1);
        i_x1 += 1;
        while i_x1 < i_x2 {
            result += spec[i_x1];
            i_x1 += 1;
        }
        if i_x1 >= spec.len() {
            i_x1 = spec.len() - 1;
        }
        result += area(spec[i_x1], x2 - i_x1 as f32);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integrate() {
        let v = vec![1.0, 2.0, 4.0, 1.123];

        // No x distance
        let c = integrate(0.0, 0.0, &v);
        assert!((c - 0.0).abs() < 0.0001);

        // No number boundary
        let c = integrate(0.25, 1.0, &v);
        assert!((c - 0.75).abs() < 0.0001);

        let c = integrate(0.0, 1.0, &v);
        assert!((c - 1.0).abs() < 0.0001);

        let c = integrate(3.75, 4.0, &v);
        assert!((c - 1.123 / 4.0).abs() < 0.0001);

        let c = integrate(0.5, 1.0, &v);
        assert!((c - 0.5).abs() < 0.0001);

        // Accross one boundary
        let c = integrate(0.75, 1.25, &v);
        assert!((c - 0.75).abs() < 0.0001);

        let c = integrate(1.8, 2.6, &v);
        assert!((c - 2.8).abs() < 0.0001);

        // Full Range
        let c = integrate(0.0, 4.0, &v);
        assert!((c - 8.123).abs() < 0.0001);
    }
}
