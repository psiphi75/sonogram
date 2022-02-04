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

use std::{fs::File, io::BufWriter};

use clap::{App, Arg};
use png::HasParameters;
use sonogram::{ColourGradient, ColourTheme, FrequencyScale, SpecOptionsBuilder};

const STR_ERR_NUM_BINS: &str = "Invalid chunk_len value, it must be an integer greater than 16";

fn main() {
    let matches = App::new("sonogram")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Simon Werner <simonwerner@gmail.com>")
        .about("sonogram - create a spectrogram as a PNG file from a wav file.")
        //
        // INPUT options
        //
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
            Arg::with_name("channel")
                .short("n")
                .long("channel")
                .value_name("NUM")
                .help("The audio channel to use")
                .default_value("1")
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
        //
        // Time domain to frequency domain transformation options
        //
        .arg(
            Arg::with_name("bins")
                .long("bins")
                .value_name("NUM")
                .help("The length of each audio chunk to process, in samples")
                .default_value("2048")
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
            Arg::with_name("freq-scale")
                .long("freq-scale")
                .value_name("TYPE")
                .help("The type of scale to use for frequency")
                .default_value("linear")
                .possible_values(&["linear", "log"])
                .takes_value(true),
        )
        .arg(
            Arg::with_name("stepsize")
                .long("stepsize")
                .value_name("SAMPLES")
                .help("The number of samples to step for each window")
                .default_value("0")
                .takes_value(true),
        )
        //
        // Output
        //
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
            Arg::with_name("legend")
                .short("l")
                .long("legend")
                .value_name("FILE")
                .help("Output the gradient legend to a PNG file")
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
            Arg::with_name("width")
                .long("width")
                .short("x")
                .value_name("PIXELS")
                .help("The width of the output image in pixels")
                .default_value("256")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("height")
                .long("height")
                .short("y")
                .value_name("PIXELS")
                .help("The height of the output image in pixels")
                .default_value("256")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("gradient")
                .long("gradient")
                .short("g")
                .value_name("NAME")
                .help("The colour gradient to implement")
                .default_value("default")
                .possible_values(&[
                    "default",
                    "audacity",
                    "rainbow",
                    "black-white",
                    "white-black",
                ])
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
    let width = match matches.value_of("width").unwrap().parse::<usize>() {
        Ok(n) => n,
        Err(_) => panic!("Invalid width value, it must be an integer greater than 1"),
    };
    let height = match matches.value_of("height").unwrap().parse::<usize>() {
        Ok(n) => n,
        Err(_) => panic!("Invalid height value, it must be an integer greater than 1"),
    };
    let freq_scale = match matches.value_of("freq-scale").unwrap() {
        "linear" => FrequencyScale::Linear,
        "log" => FrequencyScale::Log,
        _ => panic!("Invalid window function"),
    };
    let num_bins = match matches.value_of("bins").unwrap().parse::<usize>() {
        Ok(n) => {
            if n <= 16 {
                panic!("{}", STR_ERR_NUM_BINS)
            } else {
                n
            }
        }
        Err(_) => panic!("{}", STR_ERR_NUM_BINS),
    };
    let window_fn = match matches.value_of("window-function").unwrap() {
        "blackman_harris" => sonogram::blackman_harris,
        "rectangular" => sonogram::rectangular,
        "hann" => sonogram::hann_function,
        _ => panic!("Invalid window function"),
    };
    let step_size = match matches.value_of("stepsize").unwrap().parse::<usize>() {
        Ok(n) => {
            if n == 0 {
                num_bins
            } else {
                n
            }
        }
        Err(_) => panic!("Invalid stepsize value, it must be an integer greater than 0"),
    };

    let verbose = !matches.is_present("quiet");

    let mut gradient = match matches.value_of("gradient").unwrap() {
        "default" => ColourGradient::create(ColourTheme::Default),
        "audacity" => ColourGradient::create(ColourTheme::Audacity),
        "rainbow" => ColourGradient::create(ColourTheme::Rainbow),
        "black-white" => ColourGradient::create(ColourTheme::BlackWhite),
        "white-black" => ColourGradient::create(ColourTheme::WhiteBlack),
        c => panic!("Invalid colour: {}", c),
    };

    //
    // Apply the options
    //
    let spec_builder = SpecOptionsBuilder::new(num_bins)
        .load_data_from_file(std::path::Path::new(wav_file))
        .unwrap()
        .set_verbose(verbose)
        .channel(channel)
        .downsample(downsample)
        .set_window_fn(window_fn)
        .set_step_size(step_size);

    //
    // Do the spectrograph
    //
    let mut spectrograph = spec_builder.build().unwrap().compute();

    if matches.is_present("png") {
        let png_file = matches.value_of("png").unwrap();
        spectrograph
            .to_png(std::path::Path::new(png_file), freq_scale, &mut gradient)
            .unwrap()
    }

    if matches.is_present("csv") {
        let csv_file = matches.value_of("csv").unwrap();
        spectrograph
            .to_csv(std::path::Path::new(csv_file), freq_scale)
            .unwrap()
    }

    if matches.is_present("legend") {
        gradient.set_max(0.0);
        gradient.set_min(-80.0);

        let legend_file = matches.value_of("legend").unwrap();
        let width = 20;
        let height = 250;
        let legend = gradient.to_legend(width, height);

        let mut img: Vec<u8> = vec![0u8; width * height * 4];
        for (i, col) in legend.iter().take(width * height).enumerate() {
            let colour = col.to_vec();
            img[i * 4] = colour[0];
            img[i * 4 + 1] = colour[1];
            img[i * 4 + 2] = colour[2];
            img[i * 4 + 3] = colour[3];
        }

        let file = File::create(legend_file).unwrap();
        let buf = &mut BufWriter::new(file);
        let mut encoder = png::Encoder::new(buf, width as u32, height as u32);
        encoder.set(png::ColorType::RGBA).set(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&img).unwrap(); // Save
    }

    ::std::process::exit(0);
}
