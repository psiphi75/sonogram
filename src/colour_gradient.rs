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

#[derive(Clone, Copy)]
pub enum ColourTheme {
    Default,
    Audacity, // Same has the default in the audio application of the same name.
    Rainbow,
    BlackWhite, // Black background to white foreground.
    WhiteBlack, // White background to black foreground.
}

/// Colours required for a PNG file, includes the alpha channel.
#[derive(Clone, PartialEq, Debug)]
pub struct RGBAColour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl RGBAColour {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

/// ColourGradient allows you to create custom colour gradients for each
/// PNG created.
#[derive(Clone, Debug)]
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

    pub fn create(theme: ColourTheme) -> Self {
        match theme {
            ColourTheme::Default => Self::default_theme(),
            ColourTheme::Audacity => Self::audacity_theme(),
            ColourTheme::Rainbow => Self::rainbow_theme(),
            ColourTheme::BlackWhite => Self::black_white_theme(),
            ColourTheme::WhiteBlack => Self::white_black_theme(),
        }
    }

    pub fn default_theme() -> Self {
        let mut result = ColourGradient::new();
        result.add_colour(RGBAColour::new(0, 0, 0, 255)); // Black
        result.add_colour(RGBAColour::new(55, 0, 110, 255)); // Purple
        result.add_colour(RGBAColour::new(0, 0, 180, 255)); // Blue
        result.add_colour(RGBAColour::new(0, 255, 255, 255)); // Cyan
        result.add_colour(RGBAColour::new(0, 255, 0, 255)); // Green
        result
    }

    pub fn audacity_theme() -> Self {
        let mut result = ColourGradient::new();
        result.add_colour(RGBAColour::new(215, 215, 215, 255)); // Grey
        result.add_colour(RGBAColour::new(114, 169, 242, 255)); // Blue
        result.add_colour(RGBAColour::new(227, 61, 215, 255)); // Pink
        result.add_colour(RGBAColour::new(246, 55, 55, 255)); // Red
        result.add_colour(RGBAColour::new(255, 255, 255, 255)); // White
        result
    }

    pub fn rainbow_theme() -> Self {
        let mut result = ColourGradient::new();
        result.add_colour(RGBAColour::new(255, 255, 255, 255)); // White
        result.add_colour(RGBAColour::new(148, 0, 211, 255)); // Violet
        result.add_colour(RGBAColour::new(75, 0, 130, 255)); // Indigo
        result.add_colour(RGBAColour::new(0, 0, 255, 255)); // Blue
        result.add_colour(RGBAColour::new(0, 255, 0, 255)); // Green
        result.add_colour(RGBAColour::new(255, 255, 0, 255)); // Yellow
        result.add_colour(RGBAColour::new(255, 127, 0, 255)); // Orange
        result.add_colour(RGBAColour::new(255, 0, 0, 255)); // Red
        result.add_colour(RGBAColour::new(255, 255, 255, 255)); // White
        result
    }

    pub fn black_white_theme() -> Self {
        let mut result = ColourGradient::new();
        result.add_colour(RGBAColour::new(0, 0, 0, 255)); // Black
        result.add_colour(RGBAColour::new(255, 255, 255, 255)); // White
        result
    }

    pub fn white_black_theme() -> Self {
        let mut result = ColourGradient::new();
        result.add_colour(RGBAColour::new(255, 255, 255, 255)); // White
        result.add_colour(RGBAColour::new(0, 0, 0, 255)); // Black
        result
    }

    pub fn get_colour(&self, value: f32) -> RGBAColour {
        let len = self.colours.len();
        assert!(len > 1);
        assert!(self.max >= self.min);

        if value >= self.max {
            return self.colours.last().unwrap().clone();
        }
        if value <= self.min {
            return self.colours.first().unwrap().clone();
        }

        // Get the scaled values and indexes to lookup the colour
        let m = ((len - 1) as f32) / (self.max - self.min); // TODO: Precalc this value
        let scaled_value = (value - self.min) * m;
        let idx_value = scaled_value.floor() as usize;
        let ratio = scaled_value - idx_value as f32;
        let (i, j) = (idx_value, idx_value + 1);

        // Prevent over indexing after index computation
        if j >= self.colours.len() {
            return self.colours.last().unwrap().clone();
        }

        // Get the colour band
        let first = self.colours[i].clone();
        let second = self.colours[j].clone();

        RGBAColour {
            r: self.interpolate(first.r, second.r, ratio),
            g: self.interpolate(first.g, second.g, ratio),
            b: self.interpolate(first.b, second.b, ratio),
            a: self.interpolate(first.a, second.a, ratio),
        }
    }

    pub fn to_legend(&self, width: usize, height: usize) -> Vec<RGBAColour> {
        let mut result = vec![RGBAColour::new(0, 0, 0, 0); width * height];
        let step = -(self.max - self.min) / (height as f32 - 1.0);
        let mut val = self.max;
        let mut i = 0;
        for _ in 0..height {
            let col = self.get_colour(val);
            val += step;
            for _ in 0..width {
                result[i] = col.clone();
                i += 1;
            }
        }
        result
    }

    pub fn add_colour(&mut self, colour: RGBAColour) {
        self.colours.push(colour);
    }

    fn interpolate(&self, start: u8, finish: u8, ratio: f32) -> u8 {
        ((f32::from(finish) - f32::from(start)) * ratio + f32::from(start)).round() as u8
    }

    pub fn set_max(&mut self, max: f32) {
        self.max = max;
    }

    pub fn set_min(&mut self, min: f32) {
        self.min = min;
    }
}

impl Default for ColourGradient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_colour() {
        let mut gradient = ColourGradient::new();

        gradient.add_colour(RGBAColour::new(0, 0, 0, 255));
        gradient.add_colour(RGBAColour::new(255, 255, 255, 255));
        gradient.set_min(0.0);
        gradient.set_max(1.0);

        // Test two colours
        assert_eq!(gradient.get_colour(0.0), RGBAColour::new(0, 0, 0, 255));
        assert_eq!(
            gradient.get_colour(1.0),
            RGBAColour::new(255, 255, 255, 255)
        );
        assert_eq!(
            gradient.get_colour(0.5),
            RGBAColour::new(128, 128, 128, 255)
        );

        // Test three colours
        gradient.add_colour(RGBAColour::new(0, 0, 0, 255));
        assert_eq!(gradient.get_colour(0.0), RGBAColour::new(0, 0, 0, 255));
        assert_eq!(gradient.get_colour(1.0), RGBAColour::new(0, 0, 0, 255));
        assert_eq!(
            gradient.get_colour(0.5),
            RGBAColour::new(255, 255, 255, 255)
        );
        assert_eq!(gradient.get_colour(0.125), RGBAColour::new(64, 64, 64, 255));
        assert_eq!(
            gradient.get_colour(0.25),
            RGBAColour::new(128, 128, 128, 255)
        );
        assert_eq!(
            gradient.get_colour(0.75),
            RGBAColour::new(128, 128, 128, 255)
        );
    }

    #[test]
    fn test_min_max() {
        let mut gradient = ColourGradient::new();

        gradient.add_colour(RGBAColour::new(0, 0, 0, 255));
        gradient.add_colour(RGBAColour::new(255, 255, 255, 255));

        // Test two colours
        assert_eq!(gradient.get_colour(-15.0), RGBAColour::new(0, 0, 0, 255));
        assert_eq!(
            gradient.get_colour(1.0),
            RGBAColour::new(255, 255, 255, 255)
        );
        assert_eq!(
            gradient.get_colour(0.5),
            RGBAColour::new(128, 128, 128, 255)
        );
    }
}
