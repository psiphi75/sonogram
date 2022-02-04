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

pub type WindowFn = fn(usize, usize) -> f32;

pub fn rectangular(_n: usize, _samples: usize) -> f32 {
    1.0
}

pub fn hann_function(n: usize, samples: usize) -> f32 {
    0.5 * (1.0 - f32::cos((2.0 * PI * n as f32) / (samples as f32 - 1.0)))
}

pub fn blackman_harris(n: usize, samples: usize) -> f32 {
    const A0: f32 = 0.35875;
    const A1: f32 = 0.48829;
    const A2: f32 = 0.14128;
    const A3: f32 = 0.01168;

    let arg = 2.0 * PI * n as f32 / (samples as f32 - 1.0);

    A0 - A1 * f32::cos(arg) + A2 * f32::cos(2.0 * arg) - A3 * f32::cos(3.0 * arg)
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
pub fn integrate(x1: f32, x2: f32, spec: &[f32]) -> f32 {
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
