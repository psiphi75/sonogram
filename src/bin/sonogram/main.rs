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

use clap::{App, Arg};
use sonogram::{ColourGradient, RGBAColour, SpecOptionsBuilder};

const STR_ERR_OVERLAP: &str =
  "Invalid overlap value, it must be an real value greater than 0.0 and less than 0.9";
const STR_ERR_CHUNK_LEN: &str = "Invalid chunk_len value, it must be an integer greater than 16";
const STR_ERR_SCALE_RANGE: &str = "Scale value is out of range.";

fn main() {
  let matches = App::new("sonogram")
    .version(env!("CARGO_PKG_VERSION"))
    .author("Simon Werner <simonwerner@gmail.com>")
    .about("sonogram - create a spectrogram as a PNG file from a wav file.")
    .arg(
      Arg::with_name("wav")
        .short("w")
        .long("wav")
        .value_name("FILE")
        .help("The input file, a .wav")
        .required(true)
        .takes_value(true),
    )
    .arg(
      Arg::with_name("png")
        .short("p")
        .long("png")
        .value_name("FILE")
        .help("The output PNG file")
        .required(false)
        .takes_value(true),
    )
    .arg(
      Arg::with_name("csv")
        .short("c")
        .long("csv")
        .value_name("FILE")
        .help("The output CSV file")
        .required(false)
        .takes_value(true),
    )
    .arg(
      Arg::with_name("downsample")
        .short("d")
        .long("downsample")
        .value_name("NUM")
        .help("Downsample the .wav by this factor")
        .default_value("1")
        .takes_value(true),
    )
    .arg(
      Arg::with_name("channel")
        .short("n")
        .long("channel")
        .value_name("NUM")
        .help("The audio channel")
        .default_value("1")
        .required(false)
        .takes_value(true),
    )
    .arg(
      Arg::with_name("window-function")
        .short("f")
        .long("window-function")
        .value_name("FUNC NAME")
        .help("The windowing function to use")
        .possible_values(&["blackman_harris", "rectangular", "hann"])
        .default_value("rectangular")
        .takes_value(true),
    )
    .arg(
      Arg::with_name("width")
        .long("width")
        .value_name("PIXELS")
        .help("The width of the output in pixels")
        .default_value("256")
        .takes_value(true),
    )
    .arg(
      Arg::with_name("height")
        .long("height")
        .value_name("PIXELS")
        .help("The height of the output in pixels")
        .default_value("256")
        .takes_value(true),
    )
    .arg(
      Arg::with_name("chunk-len")
        .long("chunk-len")
        .value_name("NUM")
        .help("The length of each audio chunk to process, in samples")
        .default_value("2048")
        .takes_value(true),
    )
    .arg(
      Arg::with_name("overlap")
        .long("overlap")
        .value_name("OVERLAP")
        .help("The overlap between windows, in fraction")
        .default_value("0.0")
        .takes_value(true),
    )
    .arg(
      Arg::with_name("greyscale")
        .long("greyscale")
        .help("Output png as greyscale")
        .takes_value(false),
    )
    .arg(
      Arg::with_name("scale")
        .long("scale")
        .value_name("SCALE")
        .help("Scale the wav values by factor before computing spectrogram")
        .default_value("1.0")
        .takes_value(true),
    )
    .arg(
      Arg::with_name("quiet")
        .short("q")
        .long("quiet")
        .help("No console output")
        .takes_value(false),
    )
    .get_matches();

  //
  // Get the cli options
  //
  let wav_file = matches.value_of("wav").unwrap();

  if !matches.is_present("png") && !matches.is_present("csv") {
    panic!("Need to provide either a CSV or PNG output");
  }

  let downsample = match matches.value_of("downsample").unwrap().parse::<usize>() {
    Ok(n) => n,
    Err(_) => panic!("Invalid downsample value, it must be an integer greater than 1"),
  };
  let channel = match matches.value_of("channel").unwrap().parse::<u16>() {
    Ok(n) => n,
    Err(_) => panic!("Invalid channel value, it must be an integer greater than 1"),
  };
  let width = match matches.value_of("width").unwrap().parse::<u32>() {
    Ok(n) => n,
    Err(_) => panic!("Invalid width value, it must be an integer greater than 1"),
  };
  let height = match matches.value_of("height").unwrap().parse::<u32>() {
    Ok(n) => n,
    Err(_) => panic!("Invalid height value, it must be an integer greater than 1"),
  };
  let window_fn = match matches.value_of("window-function").unwrap() {
    "blackman_harris" => sonogram::blackman_harris,
    "rectangular" => sonogram::rectangular,
    "hann" => sonogram::hann_function,
    _ => panic!("Invalid window function"),
  };
  let chunk_len = match matches.value_of("chunk-len").unwrap().parse::<usize>() {
    Ok(n) => {
      if n <= 16 {
        panic!("{}", STR_ERR_CHUNK_LEN)
      } else {
        n
      }
    }
    Err(_) => panic!("{}", STR_ERR_CHUNK_LEN),
  };
  let overlap = match matches.value_of("overlap").unwrap().parse::<f32>() {
    Ok(n) => {
      if !(0.0 <= n || n < 0.9) {
        panic!("{}", STR_ERR_OVERLAP)
      } else {
        n
      }
    }
    Err(_) => panic!("{}", STR_ERR_OVERLAP),
  };
  let scale = match matches.value_of("scale").unwrap().parse::<f32>() {
    Ok(n) => {
      if !(0.000001..100000.0).contains(&n) {
        panic!("{}", STR_ERR_SCALE_RANGE)
      } else {
        n
      }
    }
    Err(_) => panic!("{}", STR_ERR_SCALE_RANGE),
  };
  let greyscale = matches.is_present("greyscale");
  let quiet = !matches.is_present("quiet");

  //
  // Apply the options
  //
  let mut spec_builder = SpecOptionsBuilder::new(width, height);
  if quiet {
    spec_builder.set_verbose();
  }
  if greyscale {
    spec_builder.set_greyscale();
  } else {
    // Colour for our plot
    let mut gradient = ColourGradient::new();
    gradient.add_colour(RGBAColour::new(0, 0, 0, 255)); // Black
    gradient.add_colour(RGBAColour::new(55, 0, 110, 255)); // Purple
    gradient.add_colour(RGBAColour::new(0, 0, 180, 255)); // Blue
    gradient.add_colour(RGBAColour::new(0, 255, 255, 255)); // Cyan
    gradient.add_colour(RGBAColour::new(0, 255, 0, 255)); // Green
    spec_builder.set_gradient(gradient);
  }

  spec_builder
    .set_window_fn(window_fn)
    .channel(channel)
    .load_data_from_file(std::path::Path::new(wav_file))
    .unwrap()
    .downsample(downsample)
    .scale(scale);

  let mut spectrograph = spec_builder.build();

  //
  // Do the spectrograph
  //
  spectrograph.compute(chunk_len, overlap);

  if matches.is_present("png") {
    let png_file = matches.value_of("png").unwrap();
    spectrograph
      .save_as_png(std::path::Path::new(png_file), false)
      .unwrap()
  }
  if matches.is_present("csv") {
    let csv_file = matches.value_of("csv").unwrap();
    spectrograph
      .save_as_csv(std::path::Path::new(csv_file), false)
      .unwrap()
  }

  ::std::process::exit(0);
}
