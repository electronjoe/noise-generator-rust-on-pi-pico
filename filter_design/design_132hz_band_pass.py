import numpy as np
from scipy.signal import butter, freqz
import matplotlib.pyplot as plt

# Define the parameters
center_frequency = 146 # Center frequency in Hz
sample_rate = 44100    # Sample rate in Hz
bandwidth = 0.2        # Bandwidth as a percentage

# Calculate the frequencies for the bandwidth
low_cutoff = center_frequency * (1 - bandwidth)
high_cutoff = center_frequency * (1 + bandwidth)

# Normalize the frequencies to the Nyquist frequency (half the sample rate)
nyquist = 0.5 * sample_rate
low = low_cutoff / nyquist
high = high_cutoff / nyquist

# Create a Butterworth filter
b, a = butter(N=1, Wn=[low, high], btype='band')

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
