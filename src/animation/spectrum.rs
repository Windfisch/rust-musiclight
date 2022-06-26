// vim: noet

use crate::animation::{Color, Animation, Result};
use crate::signal_processing::SignalProcessing;
use crate::config;

use std::rc::Rc;
use std::cell::RefCell;

use rand::Rng;

const COOLDOWN_FACTOR     : f32 = 0.960;
const RGB_EXPONENT        : f32 = 1.8;
const W_EXPONENT          : f32 = 2.2;
const FADE_FACTOR         : f32 = 0.98;
const AVG_LEDS_ACTIVATED  : f32 = 0.02;
const WHITE_EXTRA_SCALE   : f32 = 0.5;
const CONDENSATION_FACTOR : f32 = 5.0;

pub struct Spectrum
{
	colorlists   : [ [Color; config::NUM_LEDS_PER_STRIP]; config::NUM_STRIPS],
	energies: [f32; config::NUM_LEDS_TOTAL],
	sigproc: Rc<RefCell<SignalProcessing>>,
	max_energy: f32,
}

impl Animation for Spectrum
{
	fn new(sigproc: Rc<RefCell<SignalProcessing>>) -> Spectrum
	{
		Spectrum {
			colorlists: [ [Color{r: 0.0, g: 0.0, b: 0.0, w: 0.0}; config::NUM_LEDS_PER_STRIP]; config::NUM_STRIPS],
			energies: [0.0; config::NUM_LEDS_TOTAL],
			sigproc,
			max_energy: 1.0,
		}
	}

	fn init(&mut self) -> Result<()>
	{
		Ok(())
	}

	fn periodic(&mut self) -> Result<()>
	{
		let sigproc = self.sigproc.borrow();

		let max_energy = self.max_energy;

		for led in 0..config::NUM_LEDS_TOTAL
		{
			let led0_f32 = led as f32 / config::NUM_LEDS_TOTAL as f32;
			let led1_f32 = (led+1) as f32 / config::NUM_LEDS_TOTAL as f32;
			let energy = sigproc.get_energy_in_band( f32::powf(5000., led0_f32 * 0.6 + 1.0 - 0.6) , f32::powf(5000., led1_f32 * 0.6 + 1.0 - 0.6));

			let mut pitch0 = (led0_f32 * 36.0);
			while pitch0 > 12.0 {
				pitch0 -= 12.0;
			}
			let pitch1 = pitch0 + (1.0 / config::NUM_LEDS_TOTAL as f32 * 36.0);

			let mut total_energy = 0.0;
			for octave in (f32::log2(400.0) as u32)..(f32::log2(5000.0) as u32) {
				let base_freq = f32::powf(2.0, octave as f32);
				let energy = sigproc.get_energy_in_band(base_freq * f32::powf(2.0, pitch0 / 12.0), base_freq * f32::powf(2.0, pitch1 / 12.0));
				total_energy += energy;
			}

			//let total_energy = energy;


			self.energies[led] = (COOLDOWN_FACTOR * self.energies[led]).max(total_energy);

			self.colorlists[0][led] = palette( (self.energies[led] / max_energy).powf(3.0) );

			self.max_energy = self.max_energy.max(total_energy);
		}

		self.max_energy *= 0.997;

		Ok(())
	}

	fn get_colorlist(&self) -> &[ [Color; config::NUM_LEDS_PER_STRIP]; config::NUM_STRIPS]
	{
		return &self.colorlists;
	}
}

fn palette(val: f32) -> Color {
	let val = val.clamp(0.0, 1.0);

	let mut color = rainbow(val);
	if val > 0.75 {
		color.w = (val - 0.75) * 4.;
	}
	color.scale(val);

	color;

	Color { r:0., g:0., b:0., w: val}
}

fn rainbow(val: f32) -> Color {
	let val = val.clamp(0.0, 1.0);
	if val < 1./3. {
		let v = val * 3.;
		Color {
			r: 1.0 - v,
			g: v,
			b: 0.0,
			w: 0.0
		}
	}
	else if val < 2./3. {
		let v = (val - 1./3.) * 3.;
		Color {
			r: 0.0,
			g: 1.0 - v,
			b: v,
			w: 0.0
		}
	}
	else {
		let v = (val - 2./3.) * 3.;
		Color {
			r: v,
			g: 0.0,
			b: 1.0 - v,
			w: 0.0
		}
	}
}
