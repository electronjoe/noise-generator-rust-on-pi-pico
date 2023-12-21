#![no_std]
use fixed::types::I16F16;
use rand::rngs::SmallRng;
use rand::Rng;

pub struct Butterworth {
    inputs: [I16F16; 3],
    outputs: [I16F16; 2],
    input_index: usize,
    output_index: usize,
}

impl Butterworth {
    pub fn new() -> Self {
        Self {
            inputs: [I16F16::from_num(0); 3],
            outputs: [I16F16::from_num(0); 2],
            input_index: 0,
            output_index: 0,
        }
    }

    fn push_input(&mut self, input: I16F16) {
        self.inputs[self.input_index] = input;
        self.input_index = (self.input_index + 1) % self.inputs.len();
    }

    fn push_output(&mut self, output: I16F16) {
        self.outputs[self.output_index] = output;
        self.output_index = (self.output_index + 1) % self.outputs.len();
    }

    pub fn compute(&mut self, input: I16F16) -> I16F16 {
        // Push the current input
        self.push_input(input);

        // Filter coefficients
        let b = [
            I16F16::from_num(0.00414308),
            I16F16::from_num(0),
            I16F16::from_num(-0.00414308),
        ];
        let a = [
            I16F16::from_num(1), // Unused, but as intended
            I16F16::from_num(-1.99130017),
            I16F16::from_num(0.99171384),
        ];

        // Compute the output using the filter difference equation
        // y[n] = b0 * x[n] + b1 * x[n-1] + b2 * x[n-2] - a1 * y[n-1] - a2 * y[n-2]
        let y = b[0] * self.inputs[self.input_index]
            + b[1] * self.inputs[(self.input_index + 2) % 3]
            + b[2] * self.inputs[(self.input_index + 1) % 3]
            - a[1] * self.outputs[(self.output_index + 1) % 2]
            - a[2] * self.outputs[self.output_index];

        // Push the output to the buffer and return it
        self.push_output(y);

        y
    }
}

// gen_white_noise generates an I16F16 fixed point random value in the range [-1.0, 1.0].
pub fn gen_white_noise(rng: &mut SmallRng) -> I16F16 {
    // Generate a white noise sample value in range [-1.0, 1.0] in I16F16
    let rv = rng.gen::<u32>();
    // If the high bit is one, represent as a negative value.
    if rv & 0x8000_0000 == 0 {
        I16F16::from_bits((rv & 0x0000_FFFF) as i32)
    } else {
        I16F16::from_bits((rv & 0x0000_FFFF) as i32) * -1
    }
}
