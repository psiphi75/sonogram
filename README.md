# Sonogram: Wave to Spectrogram converter - in Rust

Create a sonogram\* from a wave form, or .wav file.

This crate can take a .wav file and convert it into a spectrogram. The
spectrogram can be saved as a PNG file. An example CLI progam is included
that helps convert .wav files to .png spectrograms.

The code is intended to be used as a library that can be used to convert
in-memory wave forms to a spectrograph.

\*Note: sonogram, spectrograph, spectrogram, or power spectral density
plot are common names of similar things.

Below is an example png output of sonogram.

![Sample sonogram](https://raw.githubusercontent.com/psiphi75/sonogram/master/samples/Globular-PoppingOut.png)

## Running usin the CLI

```sh
cargo run --release --bin sonogram -- --wav input.wav --png ouput.png
```

## Completing an in-memory conversion

```Rust
  let waveform: Vec<i16> = vec![];  // You'll need to fill the waveform with data.

  // Build the model
  let mut spectrograph = SpecOptionsBuilder::new(512, 128)
    .load_data_from_memory(waveform)
    .build();

  // Compute the spectrogram
  spectrograph.compute(2048, 0.8);

  // Save the spectrogram.
  let png_file = std::path::Path::new("path/to/file.png");
  spectrograph.save_as_png(&png_file, false);

```
