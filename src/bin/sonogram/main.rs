/*
 * Copyright (C) Simon Werner, 2019
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

extern crate clap;
extern crate sonogram;

use std::{fs::File, io::BufWriter, path::PathBuf};

use clap::{ArgEnum, Parser};
use png::HasParameters;
use sonogram::{ColourGradient, ColourTheme, FrequencyScale, SpecOptionsBuilder};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
enum WinFunc {
    BlackmanHarris,
    Rectangular,
    Hann,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
enum ArgColourTheme {
    Default,
    Audacity,
    Rainbow,
    BlackWhite,
    WhiteBlack,
}

impl From<ArgColourTheme> for ColourTheme {
    fn from(other: ArgColourTheme) -> ColourTheme {
        match other {
            ArgColourTheme::Default => ColourTheme::Default,
            ArgColourTheme::Audacity => ColourTheme::Audacity,
            ArgColourTheme::Rainbow => ColourTheme::Rainbow,
            ArgColourTheme::BlackWhite => ColourTheme::BlackWhite,
            ArgColourTheme::WhiteBlack => ColourTheme::WhiteBlack,
        }
    }
}

/// sonogram - create a spectrogram as a PNG file from a wav file.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    //
    // INPUT options
    //
    /// The .wav file to process
    #[clap(long, parse(from_os_str), value_name = "FILE")]
    wav: PathBuf,

    /// The audio channel to use
    #[clap(short, long, default_value_t = 1)]
    channel: u16,

    /// Downsample the .wav by this factor
    #[clap(long, default_value_t = 1)]
    downsample: usize,

    //
    // Time domain to frequency domain transformation options
    //
    /// The number of FFT bins to use
    #[clap(short, long, default_value_t = 2048)]
    bins: usize,

    /// The windowing function to use
    #[clap(arg_enum, long, default_value_t = WinFunc::Hann)]
    window_fn: WinFunc,

    /// The type of scale to use for frequency
    #[clap(long, default_value_t = String::from("linear"), value_name = "TYPE", possible_values=&["linear", "log"])]
    freq_scale: String,

    /// The number of samples to step for each window, zero mean default
    #[clap(long, default_value_t = 0)]
    stepsize: usize,
    //
    // Output
    //
    /// The output PNG file
    #[clap(long, parse(from_os_str), value_name = "FILE")]
    png: Option<PathBuf>,

    /// Output the gradient legend to a PNG file
    #[clap(long, parse(from_os_str), value_name = "FILE")]
    legend: Option<PathBuf>,

    /// The output CSV file
    #[clap(long, parse(from_os_str), value_name = "FILE")]
    csv: Option<PathBuf>,

    /// The width of the output image in pixels
    #[clap(short, long, default_value_t = 512, value_name = "PIXELS")]
    width: usize,

    /// The height of the output image in pixels
    #[clap(short, long, default_value_t = 512, value_name = "PIXELS")]
    height: usize,

    /// The colour gradient to implement
    #[clap(arg_enum, long, default_value_t = ArgColourTheme::Default, value_name = "GRADIENT")]
    gradient: ArgColourTheme,
}

fn main() {
    let args = Args::parse();

    //
    // Assert the CLI options
    //
    if args.png.is_none() && args.csv.is_none() {
        panic!("Need to provide either a CSV or PNG output");
    }

    let freq_scale = match args.freq_scale.as_str() {
        "linear" => FrequencyScale::Linear,
        "log" => FrequencyScale::Log,
        _ => panic!("Invalid window function"),
    };

    if args.bins < 16 {
        panic!(
            "Invalid bins value ({}), it must be an integer greater than 16",
            args.bins
        );
    }

    let stepsize = if args.stepsize == 0 {
        args.bins
    } else {
        args.stepsize
    };

    let window_fn = match args.window_fn {
        WinFunc::BlackmanHarris => sonogram::blackman_harris,
        WinFunc::Rectangular => sonogram::rectangular,
        WinFunc::Hann => sonogram::hann_function,
    };

    let mut gradient = ColourGradient::create(ColourTheme::from(args.gradient));

    //
    // Apply the options
    //
    let spec_builder = SpecOptionsBuilder::new(args.bins)
        .load_data_from_file(&args.wav)
        .unwrap()
        .channel(args.channel)
        .downsample(args.downsample)
        .set_window_fn(window_fn)
        .set_step_size(stepsize);

    let overlap = 1.0 - stepsize as f32 / args.bins as f32;

    println!("Computing spectrogram...");
    println!("Bins: {}", args.bins);
    println!("Overlap: {}", overlap);
    println!("Step size: {}", stepsize);

    //
    // Do the spectrograph
    //
    let mut spectrograph = spec_builder.build().unwrap().compute();

    if args.png.is_some() {
        spectrograph
            .to_png(
                &args.png.unwrap(),
                freq_scale,
                &mut gradient,
                args.width,
                args.height,
            )
            .unwrap()
    }

    if args.csv.is_some() {
        spectrograph
            .to_csv(&args.csv.unwrap(), freq_scale, args.width, args.height)
            .unwrap()
    }

    if args.legend.is_some() {
        let (min, max) = spectrograph.get_min_max();
        gradient.set_min(min);
        gradient.set_max(max);

        let width = 20;
        let height = 250;
        let legend = gradient.to_legend(width, height);

        let img = legend
            .iter()
            .flat_map(|colour| [colour.r, colour.g, colour.b, colour.a].into_iter())
            .collect::<Vec<u8>>();

        let file = File::create(&args.legend.unwrap()).unwrap();
        let buf = &mut BufWriter::new(file);
        let mut encoder = png::Encoder::new(buf, width as u32, height as u32);
        encoder.set(png::ColorType::RGBA).set(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&img).unwrap(); // Save
    }

    ::std::process::exit(0);
}
