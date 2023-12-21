import numpy as np
from scipy.signal import butter, freqz
import matplotlib.pyplot as plt

# Define the parameters
cut_off_frequency = 100 # Cut-off frequency in Hz
sample_rate = 44100     # Sample rate in Hz

# Normalize frequency for the cut-off
nyquist = 0.5 * sample_rate
cut_off_norm = cut_off_frequency / nyquist

# Create a Butterworth filter
b, a = butter(N=2, Wn=cut_off_norm, btype='low')

# Frequency response
w, h = freqz(b, a, worN=8000)
plt.plot((sample_rate * 0.5 / np.pi) * w, 20 * np.log10(abs(h)))
plt.title('Butterworth Filter Frequency Response')
plt.xlabel('Frequency [Hz]')
plt.ylabel('Magnitude [dB]')  # Updated label
plt.grid(True)
plt.xlim(1,10000)
plt.xscale('log')
plt.savefig('plot.png')

# Print the coefficients
print("Butterworth Filter Coefficients for DSP Implementation")
print("Numerator (b): ", b)
print("Denominator (a): ", a)
