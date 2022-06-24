// vim: noet

#[allow(dead_code)]

use fftw::array::AlignedVec;
use fftw::plan::*;
use fftw::types::*;
use std::f32::consts::PI;

pub struct SignalProcessing
{
	samp_rate: f32,

	fft_window: Vec<f32>,

	fft_input: AlignedVec<f32>,
	fft_output: AlignedVec<c32>,

	fft_plan: R2CPlan32,

	fft_absolute: Vec<f32>,
}

impl SignalProcessing
{
	fn hann_window(block_size: usize) -> Vec<f32>
	{
		let mut window = vec![0.0; block_size];

		for i in 0..block_size {
			window[i] = (PI * (i as f32) / (block_size as f32)).sin().powi(2);
		}

		window
	}

	pub fn new(block_size: usize, samp_rate: f32) -> fftw::error::Result<SignalProcessing>
	{
		let freq_domain_size = block_size/2 + 1;

		let s = SignalProcessing {
			samp_rate: samp_rate,
			fft_window: SignalProcessing::hann_window(block_size),
			fft_input:  AlignedVec::new(block_size),
			fft_output: AlignedVec::new(freq_domain_size),
			fft_plan:   R2CPlan::aligned(&[block_size], Flag::MEASURE)?,

			fft_absolute: vec![0.0; freq_domain_size],
		};

		Ok(s)
	}

	fn apply_window(&mut self)
	{
		self.fft_input.iter_mut()
		              .zip(self.fft_window.iter())
		              .for_each(|(s, w)| *s *= w);
	}

	pub fn import_i16_stereo(&mut self, data: &[i16]) -> std::result::Result<(), &str>
	{
		if data.len() != 2*self.fft_input.len() {
			return Err("Stereo data length does not match 2x the FFT input length.");
		}

		data.chunks_exact(2)
			.map(|channels| (channels[0] as f32 + channels[1] as f32) / 2.0 / 32768.0)
			.zip(self.fft_input.iter_mut())
			.for_each(|(c, t)| *t = c);

		self.apply_window();

		Ok(())
	}

	pub fn import_i16_mono(&mut self, data: &[i16]) -> std::result::Result<(), &str>
	{
		if data.len() != self.fft_input.len() {
			return Err("Mono data length does not match the FFT input length.");
		}

		data.iter()
			.map(|&sample| (sample as f32) / 32768.0)
			.zip(self.fft_input.iter_mut())
			.for_each(|(c, t)| *t = c);

		self.apply_window();

		Ok(())
	}

	pub fn import_i16_mono_from_iter<'a>(&mut self, mut iter: impl std::iter::Iterator<Item=&'a i16>) -> std::result::Result<(), &str>
	{
		for fft_samp in self.fft_input.iter_mut() {
			match iter.next() {
				Some(sample) => *fft_samp = *sample as f32,
				None         => return Err("Too few samples in input.")
			}
		}

		self.apply_window();

		Ok(())
	}

	pub fn is_silent(&self) -> bool
	{
		return self.fft_input.iter().sum::<f32>() == 0.0;
	}

	pub fn update_fft(&mut self) -> fftw::error::Result<()>
	{
		self.fft_plan.r2c(&mut self.fft_input, &mut self.fft_output)?;

		for (i, abs_sample) in self.fft_absolute.iter_mut().enumerate() {
			*abs_sample = self.fft_output[i].norm();
		}

		Ok(())
	}

	fn freq_to_idx(&self, freq: f32) -> usize
	{
		(freq * (self.fft_input.len() as f32) / self.samp_rate) as usize
	}

	pub fn get_energy_in_band(&self, freq_start: f32, freq_end: f32) -> f32
	{
		let start_bin = self.freq_to_idx(freq_start);
		let end_bin = self.freq_to_idx(freq_end);

		let sum: f32 = self.fft_absolute[start_bin ..= end_bin].iter().sum();
		sum / (end_bin - start_bin + 1) as f32
	}
}
