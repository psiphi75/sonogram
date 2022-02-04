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
pub use freq_scales::FrequencyScale;
pub use spec_core::SpecCompute;
pub use window_fn::*;

use std::f32;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use resize::Pixel::GrayF32;
use resize::Type::Lanczos3;
use rgb::FromSlice;

use png::HasParameters; // To use encoder.set()

pub struct Spectrogram {
    data: Vec<Vec<f32>>,
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
    ///  * `freq_scale` - Apply the log function to the frequency scale.
    ///
    pub fn to_png(
        &mut self,
        fname: &Path,
        freq_scale: FrequencyScale,
        gradient: &mut ColourGradient,
    ) -> Result<(), std::io::Error> {
        let img_height = 512;
        let result = self.spec_to_buffer(freq_scale, img_height);

        // let height = self.height;
        let width = result.len() / img_height as usize;

        println!(
            "width={}, img_height={}, result.len={}",
            width,
            img_height,
            result.len()
        );

        gradient.set_db_scale(true);
        gradient.set_min(-80.0);
        gradient.set_max(0.0);

        let mut img: Vec<u8> = vec![0u8; width * img_height * 4];
        for (i, val) in result.iter().take(width * img_height).enumerate() {
            let value = *val;
            let colour = gradient.get_colour(value).to_vec();
            img[i * 4] = colour[0];
            img[i * 4 + 1] = colour[1];
            img[i * 4 + 2] = colour[2];
            img[i * 4 + 3] = colour[3];
        }
        println!("width={}", width);

        let file = File::create(fname)?;
        let w = &mut BufWriter::new(file);
        let mut encoder = png::Encoder::new(w, width as u32, img_height as u32);
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
    ///  * `freq_scale` - Apply the log function to the frequency scale.
    ///
    pub fn to_png_in_memory(
        &mut self,
        freq_scale: FrequencyScale,
        gradient: ColourGradient,
    ) -> Result<Vec<u8>, std::io::Error> {
        let result = self.spec_to_buffer(freq_scale, 512);

        let mut img: Vec<u8> = vec![0u8; result.len() * 4];
        for (i, val) in result.iter().enumerate() {
            let colour = gradient.get_colour(*val).to_vec();
            img[i * 4] = colour[0];
            img[i * 4 + 1] = colour[1];
            img[i * 4 + 2] = colour[2];
            img[i * 4 + 3] = colour[3];
        }

        let mut pngbuf: Vec<u8> = Vec::new();

        let mut encoder = png::Encoder::new(&mut pngbuf, result.len() as u32, self.height as u32);
        encoder.set(png::ColorType::RGBA).set(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&img)?;

        // The png writer needs to be explicitly dropped
        drop(writer);
        Ok(pngbuf)
    }

    ///
    /// Save the calculated spectrogram as a CSV file.
    ///
    /// # Arguments
    ///
    ///  * `fname` - The path to the CSV to save to the filesystem.
    ///  * `freq_scale` - Apply the log function to the frequency scale.
    ///
    pub fn to_csv(
        &mut self,
        fname: &Path,
        freq_scale: FrequencyScale,
    ) -> Result<(), std::io::Error> {
        let img_height = 512;
        let result = self.spec_to_buffer(freq_scale, img_height);
        let width = result.len() / img_height as usize;

        println!(
            "width={}, img_height={}, result.len={}",
            width,
            img_height,
            result.len()
        );

        // let mut img: Vec<u8> = vec![0u8; width * img_height * 4];

        let mut writer = csv::Writer::from_path(fname)?;
        // Create the CSV header
        let mut csv_record: Vec<String> = (0..width * img_height)
            .into_iter()
            .map(|x| x.to_string())
            .collect();
        writer.write_record(&csv_record)?;

        let mut i = 0;
        for _ in 0..img_height {
            for c_rec in csv_record.iter_mut().take(width) {
                let val = result[i];
                i += 1;
                *c_rec = val.to_string();
                //self.get_real(val).to_string();
            }
            writer.write_record(&csv_record)?;
        }

        writer.flush()?; // Save

        Ok(())
    }

    ///
    /// Map the spectrogram to the output buffer.  Essentially scales the
    /// frequency to map to the vertical axis (y-axis) of the output and
    /// scale the x-axis to match the output.
    ///
    /// # Arguments
    ///
    ///  * `freq_scale` - Apply the log function to the frequency scale.
    ///
    fn spec_to_buffer(&self, freq_scale: FrequencyScale, img_height: usize) -> Vec<f32> {
        // // Only the data below 1/2 of the sampling rate (nyquist frequency)
        // // is useful

        let mut result = Vec::with_capacity(img_height * self.width);
        for h in 0..self.height {
            for w in 0..self.width {
                result.push(self.data[w][h]);
            }
        }

        let (w1, h1) = (self.width, self.height);
        let (w2, h2) = (512, 512);
        // Destination buffer. Must be mutable.
        let mut dst = vec![0.0; w2 * h2];
        // Create reusable instance.
        let mut resizer = resize::new(w1, h1, w2, h2, GrayF32, Lanczos3).unwrap();
        // Do resize without heap allocations.
        // Might be executed multiple times for different `src` or `dst`.
        resizer.resize(result.as_gray(), dst.as_gray_mut()).unwrap();
        dst
    }
}

// fn spec_to_buffer(&self, freq_scale: FrequencyScale, img_height: usize) -> Vec<f32> {
//   // Only the data below 1/2 of the sampling rate (nyquist frequency)
//   // is useful

//   // let img_height = 512;

//   let scaler = FreqScaler::create(freq_scale, self.height, img_height);
//   println!("self.height={}, self.width={}", self.height, self.width);

//   let mut result = Vec::with_capacity(img_height * self.width);
//   for h in 0..img_height {
//     let (f1, f2) = scaler.scale(h);
//     println!("{}, {}, {}, {}", h, f1, f2, self.data.len());
//     for w in 0..self.width {
//       let value = utility::integrate(f1, f2, &self.data[w]);
//       result.push(value);
//     }
//   }
//   // for h in 0..self.height {
//   //   for w in 0..self.width {
//   //     result.push(self.data[w][h]);
//   //   }
//   // }
//   println!(
//     "x,y,result.len()=({}, {}, {})",
//     self.width,
//     self.height,
//     result.len()
//   );
//   result
// }
