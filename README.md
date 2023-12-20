# Brown Noise Generator for Pimroni Pico Audio on Pi Pico in Rust

## Acknowledgements

- A great deal of the Rust in this repo is thanks to [ramenspazz](https://github.com/ramenspazz)'s [Pico_I2S](https://github.com/ramenspazz/Pico_I2S) project for this same hardware set (Pimroni Pico Audio shield on the Pi Pico) - in Rust.
- This project uses the pio I2S assembly provided by [raspberrypi/pico-extras](https://raw.githubusercontent.com/raspberrypi/pico-extras/master/src/rp2_common/pico_audio_i2s/audio_i2s.pio).
- Audacity for the general Brown Noise algorithm [Here](https://github.com/audacity/audacity/blob/236b188d6bba08ff902a7095c0425fd4a7e743de/src/effects/Noise.cpp).

## Building and Flashing

You'll need to install Rust Embedded toolchains, see the Rust Embedded book for details there.

```
cargo run --release
```

Will flash a Pi Pico that is in USB boot mode.

