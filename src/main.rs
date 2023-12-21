#![no_std]
#![no_main]

use cortex_m::singleton;
use fixed::types::I16F16;
use hal::dma::{double_buffer, single_buffer, DMAExt};
use hal::gpio::{FunctionPio0, Pin};
use hal::pac;
use hal::pio::PIOExt;
use hal::pio::ShiftDirection;
use hal::Sio;
use panic_halt as _;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rp2040_hal as hal;

/// The linker will place this boot block at the start of our program image. We
/// need this to help the ROM bootloader get our code up and running.
/// Note: This boot block is not necessary when using a rp-hal based BSP
/// as the BSPs already perform this step.
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

// constants
const XTAL_FREQ_HZ: u32 = 12_000_000u32;
const BASE_CLOCK: f32 = 125E06;
const TABLE_SIZE: usize = 220;

/// macro to split a 32bit floating point number into a u16 whole number portion and a
/// u8 fractional prortion, returned as a tuple.
macro_rules! split_float {
    ($value:expr) => {{
        let whole = $value as u16;
        let frac = (($value - whole as f32) * 256.0) as u8; // TODO: I suspect this might need to be changed to 256.0
        (whole, frac)
    }};
}

/// # Purose
/// Represents the lrck sample frequency to use, represented as its own data type to prevent
/// comparisons to numbers where ever possible.
/// # Members
/// - Freq32khz:    32khz lrck signal
/// - Freq44_1khz:  44.1khz lrck signal
/// - Freq48khz:    48khz lrck signal
/// - Freq96khz:    96khz lrck signal
/// - Freq192khz:   192khz lrck signal
/// - Freq384khz:   384khz lrck signal
enum SampleFrequency {
    #[allow(dead_code)]
    Freq32khz,
    #[allow(dead_code)]
    Freq44_1khz,
    #[allow(dead_code)]
    Freq48khz,
    #[allow(dead_code)]
    Freq96khz,
    #[allow(dead_code)]
    Freq192khz,
    #[allow(dead_code)]
    Freq384khz,
}

// Generates a sawtooth wave at 2 khz in a buffer containing 220 samples.
// Amplitude will be from -2^14 to 2^14
fn generate_sawtooth_wave(samples: &mut [u32]) {
    for i in 0..TABLE_SIZE {
        // for now ignore channel associated with high 16 bits
        let amplitude = if i < 110 {
            -1.0 + (i as f32) * (2.0 / 110.0)
        } else {
            1.0 - (i as f32 - 110.0) * (2.0 / 110.0)
        };
        let val: i16 = (amplitude * 16384.0) as i16;
        samples[i] = val as u16 as u32;
    }
}

// Generates brown noise in the low 16 bits (mono) of each buffer sample.
// Depends upon the last sample of the prior buuffer for smoothing.
// https://github.com/audacity/audacity/blob/236b188d6bba08ff902a7095c0425fd4a7e743de/src/effects/Noise.cpp#L141
// We use I16F16 to represent samples to ease the converstions from the RNG.
#[must_use]
fn generate_brown_noise(
    rng: &mut SmallRng,
    gen_num: usize,
    prior_sample: I16F16,
    samples: &mut [u32],
) -> I16F16 {
    const LEAKAGE: I16F16 = I16F16::lit("0.997");
    const SCALING: I16F16 = I16F16::lit("0.01");
    const VOLUME: I16F16 = I16F16::lit("0.25");
    let mut prior_sample = prior_sample;
    for sample in samples.iter_mut() {
        // Generate a white noise sample value in range [-1.0, 1.0] in I16F16
        let rv = rng.gen::<u32>();
        // If the high bit is one, represent as a negative value.
        let white: I16F16 = if rv & 0x8000_0000 == 0 {
            I16F16::from_bits((rv & 0x0000_FFFF) as i32)
        } else {
            I16F16::from_bits((rv & 0x0000_FFFF) as i32) * -1
        };

        let maybe_new_sample = LEAKAGE * prior_sample + white * SCALING;

        // Brown noise random walk can overflow, so here we invert the random walk if we would otherwise
        // overflow [-0.25, 0.25].
        let new_sample = if maybe_new_sample >= 0.25 || maybe_new_sample <= -0.25 {
            LEAKAGE * prior_sample - white * SCALING
        } else {
            maybe_new_sample
        };
        let new_sample = new_sample * VOLUME;

        // Scale the I16F16 so that it's integral part can be used in an i16
        let scaled_sample = new_sample * I16F16::MAX;
        *sample = (scaled_sample.to_num::<i16>()) as u32 & 0xFFFF;
        prior_sample = new_sample;
    }
    return prior_sample;
}

// Entry point to our bare-metal application.
#[rp2040_hal::entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();

    let sio = Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // configure pins for Pio
    let i2s_data: Pin<_, FunctionPio0, _> = pins.gpio9.into_function();
    let i2s_bck: Pin<_, FunctionPio0, _> = pins.gpio10.into_function();
    let i2s_lrck: Pin<_, FunctionPio0, _> = pins.gpio11.into_function();

    // PIN id for use inside of PIO
    let pin9_i2s_data = i2s_data.id().num;
    let pin10_i2s_bck: u8 = i2s_bck.id().num;
    let pin11_i2s_lrck: u8 = i2s_lrck.id().num;
    let _pin25_led: u8 = 0x19;

    // Transmit a mono or stereo I2S audio stream as stereo
    // This is 16 bits per sample; can be altered by modifying the "set" params,
    // or made programmable by replacing "set x" with "mov x, y" and using Y as a config register.
    //
    // Autopull must be enabled, with threshold set to 32.
    // Since I2S is MSB-first, shift direction should be to left.
    // Hence the format of the FIFO word is:
    //
    // | 31   :   16 | 15   :    0 |
    // | sample ws=0 | sample ws=1 |
    //
    // Data is output at 1 bit per clock. Use clock divider to adjust frequency.
    // Fractional divider will probably be needed to get correct bit clock period,
    // but for common syslck freqs this should still give a constant word select period.
    //
    // One output pin is used for the data output.
    // Two side-set pins are used. Bit 0 is clock, bit 1 is word select.
    //
    // Send 16 bit words to the PIO for mono, 32 bit words for stereo
    let program = pio_proc::pio_asm!(
        "
        .side_set 2
        
                            ;        /--- LRCLK
                            ;        |/-- BCLK
        bitloop1:           ;        ||
            out pins, 1       side 0b10
            jmp x-- bitloop1  side 0b11
            out pins, 1       side 0b00
            set x, 14         side 0b01
        
        bitloop0:
            out pins, 1       side 0b00
            jmp x-- bitloop0  side 0b01
            out pins, 1       side 0b10
        public entry_point:
            set x, 14         side 0b11
        "
    );

    // Initialize and start PIO
    let (mut pio, sm, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let target_lrck_freq = SampleFrequency::Freq44_1khz; // TODO: hardcoded for now, selection comes later

    // Find the appropriate BCK range for the desired LRCK frequency.
    // All frequencies are listed in Hertz below, abreviation Hz, units of (1/second)
    // All frequencies are pulled from Table 11. BCK Rates (MHz) by LRCK Sample Rate for PCM510xA PLL Operation
    // From the "PCM510xA 2.1 VRMS, 112/106/100 dB Audio Stereo DAC with PLL and 32-bit, 384 kHz PCM Interface" data sheet
    // We are going to use a BCK frequency at 64 times the lrck signal. The PCM5100A will accept 32 or 64 times the sampling rate.
    let (_lrck_freq, bck_freq): (f32, f32) = {
        match target_lrck_freq {
            SampleFrequency::Freq32khz => (32_000f32, 1.024E06_f32),
            SampleFrequency::Freq44_1khz => (44_100f32, 1.4112E06_f32),
            SampleFrequency::Freq48khz => (48_000f32, 1.536E06_f32),
            SampleFrequency::Freq96khz => (96_000f32, 3.072E06_f32),
            SampleFrequency::Freq192khz => (192_000f32, 6.144E06_f32),
            SampleFrequency::Freq384khz => (384_000f32, 12.288E06_f32),
        }
    };
    // let freq_offset = 1.04; // This saves the tolerance (4%)

    // clock divisor: 1/div (instructions/tick)
    // effective clock rate of PIO: 125M ticks / second * (1/div) instructions / tick => CLOCK_EFF := 125E06/div (1/seconds)
    let CK_PIO_CYCLES_PER = 2.0f32;
    let bck_data_div = (BASE_CLOCK / CK_PIO_CYCLES_PER) / bck_freq;

    // the clock divisor requires a whole and fractional divisor, so we calculate them here
    let (bck_whole, bck_frac) = split_float!(bck_data_div);

    // TODO: Calculate USB PLL settings for a UAC2 audio device

    // Set up the state machines by installing our PIO programs into the state machines and get a handle to the tx fifo on sm0
    // for transitting data to the pio from the usb line.
    let installed = pio.install(&program.program).unwrap();
    let (mut sm, _, tx) = rp2040_hal::pio::PIOBuilder::from_program(installed)
        .out_pins(pin9_i2s_data, 1)
        .side_set_pin_base(pin10_i2s_bck)
        .clock_divisor_fixed_point(bck_whole, bck_frac)
        .out_shift_direction(ShiftDirection::Left)
        .pull_threshold(32)
        .autopull(true)
        .build(sm);
    sm.set_pindirs([
        (pin9_i2s_data, hal::pio::PinDir::Output),
        (pin10_i2s_bck, hal::pio::PinDir::Output),
        (pin11_i2s_lrck, hal::pio::PinDir::Output),
    ]);

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    // Configure the clocks
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    sm.start();

    let dma = pac.DMA.split(&mut pac.RESETS);

    let message1: [u32; TABLE_SIZE] = [Default::default(); TABLE_SIZE];
    let message2: [u32; TABLE_SIZE] = [Default::default(); TABLE_SIZE];

    // Transfer two single messages via DMA.
    let tx_buf1 = singleton!(: [u32; TABLE_SIZE] = message1).unwrap();
    let tx_buf2 = singleton!(: [u32; TABLE_SIZE] = message2).unwrap();
    let mut small_rng = SmallRng::seed_from_u64(0xfeedbeeffeedbeef_u64);
    let mut prior_sample = I16F16::lit("0.0");
    let mut gen_num: usize = 0;
    prior_sample = generate_brown_noise(&mut small_rng, gen_num, prior_sample, tx_buf1);
    gen_num += 1;
    prior_sample = generate_brown_noise(&mut small_rng, gen_num, prior_sample, tx_buf2);
    gen_num += 1;
    let tx_transfer1 = single_buffer::Config::new(dma.ch0, tx_buf1, tx).start();
    let (ch0, tx_buf1, tx) = tx_transfer1.wait();
    let tx_transfer2 = single_buffer::Config::new(dma.ch1, tx_buf2, tx).start();
    let (ch1, tx_buf2, tx) = tx_transfer2.wait();

    // Chain some buffers together for continuous transfers
    let mut tx_transfer = double_buffer::Config::new((ch0, ch1), tx_buf1, tx)
        .start()
        .read_next(tx_buf2);
    // Here I create a third buffer, because I believe I need three in order to support
    // double-buffer ping/pong using read_next below.
    let mut next_buf = singleton!(: [u32; TABLE_SIZE] = message1).unwrap();
    loop {
        if tx_transfer.is_done() {
            // Here we generate new brown noise while the last DMA (triggered by read_next below)
            // is still doing its thing.
            prior_sample = generate_brown_noise(&mut small_rng, gen_num, prior_sample, next_buf);
            gen_num += 1;
            // wait is a blocking call, returns when tx_transfer is complete
            let (tx_buf, next_tx_transfer) = tx_transfer.wait();
            // read_next is IMO confusing named - but from our point of view it's toggling
            // what DMA channel is used and specifying next_buf for the new transfer,
            // finally it begins the new DMA channel that uses next_buf.
            tx_transfer = next_tx_transfer.read_next(next_buf);
            next_buf = tx_buf;
        }
    }
}
