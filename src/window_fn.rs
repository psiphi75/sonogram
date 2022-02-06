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
