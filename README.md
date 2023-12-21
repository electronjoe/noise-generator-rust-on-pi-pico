# Brown Noise Generator for Pimoroni Pico Audio on Pi Pico in Rust

## Acknowledgements

- A great deal of the Rust in this repo is thanks to [ramenspazz](https://github.com/ramenspazz)'s [Pico_I2S](https://github.com/ramenspazz/Pico_I2S) project for this same hardware set (Pimoroni Pico Audio shield on the Pi Pico) - in Rust.
- This project uses the pio I2S assembly provided by [raspberrypi/pico-extras](https://raw.githubusercontent.com/raspberrypi/pico-extras/master/src/rp2_common/pico_audio_i2s/audio_i2s.pio).
- Audacity for the general Brown Noise algorithm [Here](https://github.com/audacity/audacity/blob/236b188d6bba08ff902a7095c0425fd4a7e743de/src/effects/Noise.cpp).

## Building and Flashing

You'll need to install Rust Embedded toolchains, see the Rust Embedded book for details there.

```
cargo run --release
```

Will flash a Pi Pico that is in USB boot mode.

## Filter Design

Butterworth first order filter

- center_frequency = 146 # Center frequency in Hz
- sample_rate = 44100    # Sample rate in Hz
- bandwidth = 0.2        # Bandwidth as a percentage

Butterworth Filter Coefficients for DSP Implementation:
- Numerator (b):  [ 0.00414308  0.         -0.00414308]
- Denominator (a):  [ 1.         -1.99130017  0.99171384]

y[n]=b[0]×x[n]+b[1]×x[n−1]+⋯+a[1]×y[n−1]+…
- where x[n] is your input signal, and y[n] is your filtered output signal.

## Host Testing

I've authored a test binary to generate WAV file outputs for debug iteration in the host environment.

```
cargo test --test main --target x86_64-unknown-linux-gnu
```