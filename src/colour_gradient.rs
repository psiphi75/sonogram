/*
 * Copyright (C) Simon Werner, 2019
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

/// Colours required for a PNG file, includes the alpha channel.
#[derive(Clone)]
pub struct RGBAColour {
  r: u8,
  g: u8,
  b: u8,
  a: u8,
}

impl RGBAColour {
  pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
    Self { r, g, b, a }
  }

  pub fn to_vec(&self) -> Vec<u8> {
    vec![self.r, self.g, self.b, self.a]
  }
}

/// ColourGradient allows you to create custom colour gradients for each
/// PNG created.
pub struct ColourGradient {
  colours: Vec<RGBAColour>,
  min: f32,
  max: f32,
}

impl ColourGradient {
  pub fn new() -> Self {
    Self {
      colours: vec![],
      min: 0.0,
      max: 1.0,
    }
  }

  pub fn get_colour(&self, value: f32) -> RGBAColour {
    assert!(self.colours.len() > 1);

    if value >= self.max {
      return self.colours.last().unwrap().clone();
    }
    let mut ratio = value / self.max;
    let width = 1.0 / (self.colours.len() as f32 - 1.0);
    let mut i = 0;

    // Find the "bin"
    while ratio > width {
      ratio -= width;
      i += 1;
    }

    ratio *= (self.colours.len() - 1) as f32;

    assert!(0.0 <= ratio);
    assert!(ratio <= 1.0);
    assert!(i < self.colours.len());

    let first = self.colours[i].clone();
    let second = self.colours[i + 1].clone();

    RGBAColour {
      r: self.interpolate(first.r, second.r, ratio),
      g: self.interpolate(first.g, second.g, ratio),
      b: self.interpolate(first.b, second.b, ratio),
      a: 255,
    }
  }

  pub fn add_colour(&mut self, colour: RGBAColour) {
    self.colours.push(colour);
  }

  fn interpolate(&self, start: u8, finish: u8, ratio: f32) -> u8 {
    ((f32::from(finish) - f32::from(start)) * ratio + start as f32).round() as u8
  }

  pub fn set_max(&mut self, max: f32) {
    self.max = max
  }

  pub fn set_min(&mut self, min: f32) {
    self.min = min
  }
}
