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
use sonogram::{spectrograph::SpecOptionsBuilder, utility};

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
        .required(true)
        .takes_value(true),
    )
    .get_matches();

  let wav_file = matches.value_of("wav").unwrap();
  let png_file = matches.value_of("png").unwrap();

  let mut spectrograph = SpecOptionsBuilder::new(1024, 256)
    .set_window_fn(utility::blackman_harris)
    .load_data_from_file(&std::path::Path::new(wav_file))
    .build();

  spectrograph.compute(2048, 0.8);
  spectrograph.save_as_png(&std::path::Path::new(png_file), false);

  ::std::process::exit(0);
}
