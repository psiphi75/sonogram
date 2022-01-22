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

use std::f32;
use std::f32::consts::PI;

use num_complex::Complex;

pub fn reverse_bits(val: usize, power: usize) -> usize {
  let mut reversed = 0;

  for i in 0..power {
    let cur_bit = if (1 << i) & val > 0 { 1 } else { 0 };
    reversed |= cur_bit << (power - i - 1);
  }

  reversed
}

pub fn rectangular(_n: u32, _samples: u32) -> f32 {
  1.0
}

pub fn hann_function(n: u32, samples: u32) -> f32 {
  0.5 * (1.0 - f32::cos((2.0 * PI * n as f32) / (samples as f32 - 1.0)))
}

pub fn blackman_harris(n: u32, samples: u32) -> f32 {
  const A0: f32 = 0.35875;
  const A1: f32 = 0.48829;
  const A2: f32 = 0.14128;
  const A3: f32 = 0.01168;

  let arg = 2.0 * PI * n as f32 / (samples as f32 - 1.0);

  A0 - A1 * f32::cos(arg) + A2 * f32::cos(2.0 * arg) - A3 * f32::cos(3.0 * arg)
}

pub fn pad_to_power2(signal: &mut Vec<Complex<f32>>, min_len: usize) -> usize {
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

pub fn pad(signal: &mut Vec<Complex<f32>>, new_len: usize) {
  if new_len > signal.len() {
    signal.resize_with(new_len, || Complex::new(0.0, 0.0));
  }
}

///
/// Integrate `spec` from `y1` to `y2`, where `y1` and `y2` are
/// floating point indicies where we take the fractional component into
/// account as well.
///
/// Integration is uses simple linear interpolation.
///
/// # Arguments
///
/// * `y1` - The fractional index that points to `spec`.
/// * `y2` - The fractional index that points to `spec`.
/// * `spec` - The values that require integration.
///
/// # Returns
///
/// The integrated complex value.
///
pub fn integrate(y1: f32, y2: f32, spec: &[Complex<f32>]) -> Complex<f32> {
  let i_y1 = y1.floor() as usize;
  let i_y2 = y2.floor() as usize;
  let f_y1 = y1.fract();
  let f_y2 = y2.fract();

  // Calculate the ratio from
  let ratio = |v1, v2, frac| (v2 - v1) * (frac);

  if i_y1 == i_y2 {
    // Sub-cell integration
    ratio(spec[i_y1], spec[i_y1 + 1], f_y2 - f_y1)
  } else {
    // Need to integrate from y1 to y2 over multiple indicies.
    let mut result = ratio(spec[i_y1], spec[i_y1 + 1], 1.0 - f_y1);
    for c in spec.iter().take(i_y2).skip(i_y1 + 1) {
      result += c;
    }
    result += ratio(spec[i_y2], spec[i_y2 + 1], f_y2);
    result
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_integrate() {
    let v = vec![
      Complex::new(0.0, 0.0),
      Complex::new(1.0, 0.0),
      Complex::new(2.0, 0.0),
      Complex::new(4.0, 0.0),
    ];

    // No number boundary
    let c = integrate(0.6, 0.8, &v);
    assert!((c.norm() - 0.2) < 0.0001);

    let c = integrate(1.000001, 1.999999, &v);
    assert!((c.norm() - 1.0) < 0.001);

    // One number boundary
    let c = integrate(1.6, 2.2, &v);
    assert!((c.norm() - 0.8) < 0.0001);

    // Two number boundary
    let c = integrate(0.00001, 2.99999, &v);
    assert!((c.norm() - 7.0) < 0.0001);
  }
}
