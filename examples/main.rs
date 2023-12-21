use fixed::types::I16F16;
use noise_generator::{gen_white_noise, Butterworth};
use rand::rngs::SmallRng;
use rand::SeedableRng;
use riff_wave::WaveWriter;
use std::f32::consts::PI;
use std::fs::File;
use std::io::BufWriter;

fn main() {
    let mut butterworth = Butterworth::new();

    let mut small_rng = SmallRng::seed_from_u64(0xfeedbeeffeedbeef_u64);

    const SAMPLE_RATE: u32 = 44100;
    const SAMPLE_COUNT: u32 = SAMPLE_RATE * 2;

    let file = File::create("examples/hello.wav").unwrap();
    let writer = BufWriter::new(file);

    let mut wave_writer = WaveWriter::new(1, SAMPLE_RATE, 16, writer).unwrap();

    for n in 0..SAMPLE_COUNT {
        // Generate a white noise sample value in range [-1.0, 1.0] in I16F16
        let white: I16F16 = gen_white_noise(&mut small_rng);

        let sample = butterworth.compute(white);
        print!("{}->{}\n", white, sample);
        let scaled_sample = sample * I16F16::MAX;
        let sample_i16 = scaled_sample.to_num::<i16>();
        wave_writer.write_sample_i16(sample_i16).unwrap();
    }
}
