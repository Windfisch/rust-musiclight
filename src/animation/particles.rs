// vim: noet

use crate::animation::{Color, Animation, Result};
use crate::signal_processing::SignalProcessing;
use crate::config;

use std::rc::Rc;
use std::cell::RefCell;

use rand::Rng;

const COOLDOWN_FACTOR     : f32 = 0.99980;
const RGB_EXPONENT        : f32 = 1.8;
const W_EXPONENT          : f32 = 2.2;
const FADE_FACTOR         : f32 = 0.98;
const AVG_LEDS_ACTIVATED  : f32 = 0.02;
const WHITE_EXTRA_SCALE   : f32 = 0.5;
const CONDENSATION_FACTOR : f32 = 5.0;

pub struct Particles
{
	energy       : [ [Color; config::NUM_LEDS_PER_STRIP]; config::NUM_STRIPS],
	max_energy   : Color,

	colorlists   : [ [Color; config::NUM_LEDS_PER_STRIP]; config::NUM_STRIPS],

	sigproc: Rc<RefCell<SignalProcessing>>,
}

impl Animation for Particles
{
	fn new(sigproc: Rc<RefCell<SignalProcessing>>) -> Particles
	{
		Particles {
			energy:     [ [Color{r: 0.0, g: 0.0, b: 0.0, w: 0.0}; config::NUM_LEDS_PER_STRIP]; config::NUM_STRIPS],
			max_energy: Color{r: 1.0, g: 1.0, b: 1.0, w: 1.0},
			colorlists: [ [Color{r: 0.0, g: 0.0, b: 0.0, w: 0.0}; config::NUM_LEDS_PER_STRIP]; config::NUM_STRIPS],
			sigproc: sigproc,
		}
	}

	fn init(&mut self) -> Result<()>
	{
		Ok(())
	}

	fn periodic(&mut self) -> Result<()>
	{
		let sigproc = self.sigproc.borrow();

		// extract frequency band energies
		let cur_energy = Color{
			r: sigproc.get_energy_in_band(    0.0,   400.0),
			g: sigproc.get_energy_in_band(  400.0,  4000.0),
			b: sigproc.get_energy_in_band( 4000.0, 12000.0),
			w: sigproc.get_energy_in_band(12000.0, 22000.0)};

		// track the maximum energy with cooldown
		self.max_energy.r *= COOLDOWN_FACTOR;
		if cur_energy.r > self.max_energy.r {
			self.max_energy.r = cur_energy.r;
		}

		self.max_energy.g *= COOLDOWN_FACTOR;
		if cur_energy.g > self.max_energy.g {
			self.max_energy.g = cur_energy.g;
		}

		self.max_energy.b *= COOLDOWN_FACTOR;
		if cur_energy.b > self.max_energy.b {
			self.max_energy.b = cur_energy.b;
		}

		self.max_energy.w *= COOLDOWN_FACTOR;
		if cur_energy.w > self.max_energy.w {
			self.max_energy.w = cur_energy.w;
		}

		// fade all LEDs towards black
		for strip in 0..config::NUM_STRIPS {
			for led in 0..config::NUM_LEDS_PER_STRIP {
				self.energy[strip][led].scale(FADE_FACTOR);
			}
		}

		// distribute the energy for each color
		let new_energy = Color{
			r: (cur_energy.r / self.max_energy.r).powf(RGB_EXPONENT),
			g: (cur_energy.g / self.max_energy.g).powf(RGB_EXPONENT),
			b: (cur_energy.b / self.max_energy.b).powf(RGB_EXPONENT),
			w: (cur_energy.w / self.max_energy.w).powf(W_EXPONENT),
		};

		let mut remaining_energy = new_energy;
		remaining_energy.scale(AVG_LEDS_ACTIVATED * config::NUM_LEDS_TOTAL as f32);

		let mut rng = rand::thread_rng();

		for coloridx in 0..=3 {
			let new_energy_ref = new_energy.ref_by_index(coloridx).unwrap();
			let rem_energy_ref = remaining_energy.ref_by_index_mut(coloridx).unwrap();

			while *rem_energy_ref > 0.0 {
				let mut rnd_energy = rng.gen::<f32>() * (*new_energy_ref) * CONDENSATION_FACTOR;

				let rnd_strip = rng.gen_range(0..config::NUM_STRIPS);
				let rnd_led   = rng.gen_range(0..config::NUM_LEDS_PER_STRIP);

				if rnd_energy > *rem_energy_ref {
					rnd_energy = *rem_energy_ref;
					*rem_energy_ref = 0.0;
				} else {
					*rem_energy_ref -= rnd_energy;
				}

				let led_ref = self.energy[rnd_strip][rnd_led].ref_by_index_mut(coloridx).unwrap();
				*led_ref += rnd_energy;
			}
		}

		// color post-processing
		self.colorlists = self.energy;

		for strip in 0..config::NUM_STRIPS {
			for led in 0..config::NUM_LEDS_PER_STRIP {
				self.colorlists[strip][led].w *= WHITE_EXTRA_SCALE;

				self.colorlists[strip][led].limit();
			}
		}

		Ok(())
	}

	fn get_colorlist(&self) -> &[ [Color; config::NUM_LEDS_PER_STRIP]; config::NUM_STRIPS]
	{
		return &self.colorlists;
	}
}
