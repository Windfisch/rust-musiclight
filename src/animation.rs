// vim: noet

use std::fmt;
use std::error::Error as StdError;

use std::rc::Rc;
use std::cell::RefCell;

use crate::config;
use crate::signal_processing::SignalProcessing;

type Result<T> = std::result::Result<T, AnimationError>;

/////////// Error Type and Implementation ////////////

#[derive(Debug)]
pub enum AnimationError
{
	ErrorMessage(std::string::String),
}

impl fmt::Display for AnimationError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			AnimationError::ErrorMessage(s) => f.write_fmt(format_args!("Message({})", s))?,
		};

		Ok(())
	}
}

impl StdError for AnimationError {
	fn description(&self) -> &str {
		match *self {
			AnimationError::ErrorMessage(_) => "Error Message",
		}
	}
}

/////////// Helper Structs ////////////

#[derive(Copy, Clone)]
pub struct Color
{
	pub r: f32,
	pub g: f32,
	pub b: f32,
	pub w: f32
}

impl Color
{
	pub fn scale(&mut self, factor: f32)
	{
		self.r *= factor;
		self.g *= factor;
		self.b *= factor;
		self.w *= factor;
	}

	pub fn scaled_copy(&self, factor: f32) -> Color
	{
		let mut c = *self;

		c.scale(factor);
		c
	}

	pub fn add(&mut self, other: &Color)
	{
		self.r += other.r;
		self.g += other.g;
		self.b += other.b;
		self.w += other.w;
	}

	fn _limit_component(c: &mut f32)
	{
		if *c > 1.0 {
			*c = 1.0;
		} else if *c < 0.0 {
			*c = 0.0;
		}
	}

	pub fn limit(&mut self)
	{
		Color::_limit_component(&mut self.r);
		Color::_limit_component(&mut self.g);
		Color::_limit_component(&mut self.b);
		Color::_limit_component(&mut self.w);
	}

	pub fn ref_by_index_mut(&mut self, i: usize) -> Option<&mut f32>
	{
		match i {
			0 => Some(&mut self.r),
			1 => Some(&mut self.g),
			2 => Some(&mut self.b),
			3 => Some(&mut self.w),
			_ => None
		}
	}

	pub fn ref_by_index(&self, i: usize) -> Option<&f32>
	{
		match i {
			0 => Some(&self.r),
			1 => Some(&self.g),
			2 => Some(&self.b),
			3 => Some(&self.w),
			_ => None
		}
	}
}

/////////// Animation Trait ////////////

pub trait Animation {
	fn new(sigproc: Rc<RefCell<SignalProcessing>>) -> Self;

	fn init(&mut self) -> Result<()>;
	fn periodic(&mut self) -> Result<()>;

	fn get_colorlist(&self) -> &[ [Color; config::NUM_LEDS_PER_STRIP]; config::NUM_STRIPS];
}

/////////// Animation implementations ////////////

pub mod particles
{
	use crate::animation::{Color, Animation, Result};
	use crate::signal_processing::SignalProcessing;
	use crate::config;

	use std::rc::Rc;
	use std::cell::RefCell;

	use rand::Rng;

	const COOLDOWN_FACTOR     : f32 = 0.99995;
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
}

pub mod sparkles
{
	use crate::animation::{Color, Animation, Result};
	use crate::signal_processing::SignalProcessing;
	use crate::config;

	use std::rc::Rc;
	use std::cell::RefCell;
	use std::collections::VecDeque;

	use rand::Rng;

	const COOLDOWN_FACTOR     : f32 = 0.99995;
	const RGB_EXPONENT        : f32 = 1.5;
	const W_EXPONENT          : f32 = 2.2;
	const FADE_FACTOR         : f32 = 0.97;
	const AVG_LEDS_ACTIVATED  : f32 = 0.03;
	const WHITE_EXTRA_SCALE   : f32 = 0.3;
	const CONDENSATION_FACTOR : f32 = 5.0;

	const SPARK_FADE_STEP     : f32 = 2.500 / config::FPS_ANIMATION;
	const SPARK_VSPEED_MIDS   : f32 = 1.000 * config::NUM_LEDS_PER_STRIP as f32 / config::FPS_ANIMATION;
	const SPARK_VSPEED_HIGHS  : f32 = 0.800 * config::NUM_LEDS_PER_STRIP as f32 / config::FPS_ANIMATION;
	const SPARK_VSPEED_XHIGHS : f32 = 0.500 * config::NUM_LEDS_PER_STRIP as f32 / config::FPS_ANIMATION;

	/*
	 * A spark is a point of light that can move vertically along the LED strips.
	 */
	struct Spark
	{
		pub vspeed:     f32, // LEDs per frame
		pub brightness: f32,
		pub color:      Color,

		strip:      u16,
		led:        f32,

		has_expired: bool,
	}

	impl Spark
	{
		pub fn new(vspeed: f32, brightness: f32, color: Color, strip: u16, led: f32) -> Spark
		{
			Spark {
				vspeed: vspeed,
				brightness: brightness,
				color: color,
				strip: strip,
				led: led,
				has_expired: false
			}
		}

		pub fn update(&mut self)
		{
			if self.has_expired {
				return;
			}

			self.led += self.vspeed;
			self.brightness -= SPARK_FADE_STEP;

			if (self.led >= config::NUM_LEDS_PER_STRIP as f32) || (self.led <= -1.0) {
				// moved outside of the LED array -> no need to update this any more
				self.has_expired = true;
			}

			if self.brightness <= 0.0 {
				// moved outside of the LED array -> no need to update this any more
				self.has_expired = true;
			}
		}

		pub fn has_expired(&self) -> bool
		{
			self.has_expired
		}

		pub fn render(&self, colorlists: &mut [ [Color; config::NUM_LEDS_PER_STRIP]; config::NUM_STRIPS])
		{
			if self.has_expired {
				// do not render if this Spark has expired
				return;
			}

			let fract_led = self.led - self.led.floor();

			let led1_idx = self.led.floor() as i32;
			let led2_idx = self.led.ceil() as usize;

			let led1_color = self.color.scaled_copy(fract_led * self.brightness);
			let led2_color = self.color.scaled_copy((1.0 - fract_led) * self.brightness);

			if led1_idx >= 0 {
				colorlists[self.strip as usize][led1_idx as usize].add(&led1_color);
			}

			if led2_idx < config::NUM_LEDS_PER_STRIP {
				colorlists[self.strip as usize][led2_idx as usize].add(&led2_color);
			}
		}
	}

	pub struct Sparkles
	{
		max_energy   : Color,

		sparks : VecDeque<Spark>,

		colorlists   : [ [Color; config::NUM_LEDS_PER_STRIP]; config::NUM_STRIPS],

		sigproc: Rc<RefCell<SignalProcessing>>,
	}

	impl Animation for Sparkles
	{
		fn new(sigproc: Rc<RefCell<SignalProcessing>>) -> Sparkles
		{
			Sparkles {
				max_energy: Color{r: 1.0, g: 1.0, b: 1.0, w: 1.0},
				sparks: VecDeque::with_capacity(1024),
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
					self.colorlists[strip][led].scale(FADE_FACTOR);
				}
			}

			// distribute the energy for each color
			let new_energy = Color{
				r: (cur_energy.r / self.max_energy.r).powf(RGB_EXPONENT),
				g: (cur_energy.g / self.max_energy.g).powf(RGB_EXPONENT),
				b: (cur_energy.b / self.max_energy.b).powf(RGB_EXPONENT),
				w: (cur_energy.w / self.max_energy.w).powf(W_EXPONENT),
			};

			let mut remaining_energy = new_energy.r;
			remaining_energy *= AVG_LEDS_ACTIVATED * config::NUM_LEDS_TOTAL as f32;

			let mut rng = rand::thread_rng();

			// Red (bass) uses exactly the same algorithm as for the “Particles” animation.
			while remaining_energy > 0.0 {
				let mut rnd_energy = rng.gen::<f32>() * new_energy.r * CONDENSATION_FACTOR;

				let rnd_strip = rng.gen_range(0..config::NUM_STRIPS);
				let rnd_led   = rng.gen_range(0..config::NUM_LEDS_PER_STRIP);

				if rnd_energy > remaining_energy {
					rnd_energy = remaining_energy;
					remaining_energy = 0.0;
				} else {
					remaining_energy -= rnd_energy;
				}

				self.colorlists[rnd_strip][rnd_led].r += rnd_energy;
			}

			// update all existing sparks
			self.sparks.iter_mut().for_each(|x| x.update());

			// Create green sparks for middle frequencies.
			// They originate in the center and can go both up and down from there.
			self.sparks.push_back(Spark::new(
					match rng.gen::<bool>() {
						true => SPARK_VSPEED_MIDS,
						false => -SPARK_VSPEED_MIDS,
					},
					new_energy.g,
					Color{r: 0.0, g: 1.0, b: 0.0, w: 0.0},
					rng.gen_range(0..config::NUM_STRIPS) as u16,
					(config::NUM_LEDS_PER_STRIP as f32 / 2.0) - 0.5));

			// Create blue sparks for high frequencies.
			// They originate either in the top, moving down, or in the bottom, moving up
			{
				let start_from_top = rng.gen::<bool>();

				let start_led = match start_from_top {
					true => config::NUM_LEDS_PER_STRIP-1,
					false => 0} as f32;

				let vspeed = match start_from_top {
					true => -SPARK_VSPEED_HIGHS,
					false => SPARK_VSPEED_HIGHS};

				self.sparks.push_back(Spark::new(
						vspeed,
						new_energy.b,
						Color{r: 0.0, g: 0.0, b: 1.0, w: 0.0},
						rng.gen_range(0..config::NUM_STRIPS) as u16,
						start_led));
			}

			// Create white sparks for very high frequencies.
			// They originate either in the top, moving down, or in the bottom, moving up
			{
				let start_from_top = rng.gen::<bool>();

				let start_led = match start_from_top {
					true => config::NUM_LEDS_PER_STRIP-1,
					false => 0} as f32;

				let vspeed = match start_from_top {
					true => -SPARK_VSPEED_XHIGHS,
					false => SPARK_VSPEED_XHIGHS};

				self.sparks.push_back(Spark::new(
						vspeed,
						new_energy.w * WHITE_EXTRA_SCALE,
						Color{r: 0.0, g: 0.0, b: 0.0, w: 1.0},
						rng.gen_range(0..config::NUM_STRIPS) as u16,
						start_led));
			}

			// remove expired sparks in the beginning of the deque
			while self.sparks.front().map_or(false, |s| s.has_expired()) {
				self.sparks.pop_front();
			}

			// render all remaining sparks
			for spark in self.sparks.iter() {
				spark.render(&mut self.colorlists);
			}

			// color post-processing
			for strip in 0..config::NUM_STRIPS {
				for led in 0..config::NUM_LEDS_PER_STRIP {
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
}
