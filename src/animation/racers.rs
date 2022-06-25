// vim: noet

use crate::animation::{Color, Animation, Result};
use crate::signal_processing::SignalProcessing;
use crate::config;

use std::rc::Rc;
use std::cell::RefCell;

use rand::Rng;

const COOLDOWN_FACTOR         : f32 = 0.99980;
const RGB_EXPONENT            : f32 = 1.5;
const W_EXPONENT              : f32 = 2.2;
const W_SCALE                 : f32 = 0.3;
const ENERGY_FILTER_ALPHA     : f32 = 0.20;
const BRIGHTNESS_FILTER_ALPHA : f32 = 0.002;

const NUM_RACERS_R        : usize = 10 * config::NUM_LEDS_TOTAL / 300;
const NUM_RACERS_G        : usize = 10 * config::NUM_LEDS_TOTAL / 300;
const NUM_RACERS_B        : usize = 10 * config::NUM_LEDS_TOTAL / 300;

const RACER_MIN_SPEED_R        : f32 =  0.5 / config::FPS_ANIMATION;
const RACER_MAX_SPEED_R        : f32 = 80.0 / config::FPS_ANIMATION;
const RACER_MIN_BRIGHTNESS_R   : f32 = 0.01;
const RACER_MAX_BRIGHTNESS_R   : f32 = 1.00;

const RACER_MIN_SPEED_G        : f32 =  0.5 / config::FPS_ANIMATION;
const RACER_MAX_SPEED_G        : f32 = 80.0 / config::FPS_ANIMATION;
const RACER_MIN_BRIGHTNESS_G   : f32 = 0.01;
const RACER_MAX_BRIGHTNESS_G   : f32 = 1.00;

const RACER_MIN_SPEED_B        : f32 =  0.5 / config::FPS_ANIMATION;
const RACER_MAX_SPEED_B        : f32 = 80.0 / config::FPS_ANIMATION;
const RACER_MIN_BRIGHTNESS_B   : f32 = 0.01;
const RACER_MAX_BRIGHTNESS_B   : f32 = 1.00;

const SPEED_SCALE_RANGE        : f32 = 0.10;

fn dbg_bar(min: f32, current: f32, max: f32)
{
	const LEN: usize = 60;
	let mut bar = ['.'; LEN];

	let minpos = ((LEN as f32) * min / max) as usize;
	let curpos = ((LEN as f32) * current / max) as usize;

	for idx in minpos..LEN {
		bar[idx] = '-';
	}

	if curpos < LEN {
		bar[curpos] = '#';
	}

	print!("{}", bar.iter().collect::<String>());
}

/*
 * A racer is a point of light that can move along the LED strips.
 */
struct Racer
{
	min_speed:  f32, // LEDs per frame
	max_speed:  f32, // LEDs per frame
	direction:  i8,  // either +1 or -1

	min_brightness: f32,
	max_brightness: f32,

	color: Color,

	pos: f32,

	brightness: f32,
	flare_brightness: f32,
}

impl Racer
{
	pub fn new(min_speed: f32, max_speed: f32, min_brightness: f32, max_brightness: f32, color: Color, start_pos: f32, direction: i8) -> Racer
	{
		Racer {
			min_speed: min_speed,
			max_speed: max_speed,
			min_brightness: min_brightness,
			max_brightness: max_brightness,
			direction: direction,
			color: color,
			pos: start_pos,
			brightness: min_brightness,
			flare_brightness: 0.0,
		}
	}

	fn _pos2ledstrip(pos: i32) -> (i32, i32)
	{
		let strip = pos / (config::NUM_LEDS_PER_STRIP as i32);

		let mut led = pos % (config::NUM_LEDS_PER_STRIP as i32);

		if (strip % 2) == 1 {
			led = (config::NUM_LEDS_PER_STRIP as i32) - led - 1;
		}

		(strip, led)
	}

	pub fn update(&mut self, speed: f32, brightness: f32, flare_brightness: f32)
	{
		// move along the strip
		let cur_speed = self.min_speed + speed * (self.max_speed - self.min_speed);

		self.pos += (self.direction as f32) * cur_speed;

		let maxpos = config::NUM_LEDS_TOTAL as f32;

		// if the end is reached, reverse the direction
		if self.pos >= maxpos {
			self.direction = -1;
			self.pos = 2.0 * maxpos - self.pos;
		} else if self.pos <= 0.0 {
			self.direction = 1;
			self.pos = -self.pos;
		}

		self.brightness = brightness;
		self.flare_brightness = flare_brightness;
	}

	pub fn render(&self, colorlists: &mut [ [Color; config::NUM_LEDS_PER_STRIP]; config::NUM_STRIPS])
	{
		let brightness = self.min_brightness + self.brightness * (self.max_brightness - self.min_brightness);

		let fract_led = self.pos - self.pos.floor();

		let led1_idx = self.pos.floor() as i32;
		let led2_idx = self.pos.ceil() as i32;

		let mut color = self.color;
		color.w += self.flare_brightness;

		let led1_color = color.scaled_copy((1.0 - fract_led) * brightness);
		let led2_color = color.scaled_copy(fract_led * brightness);

		if led1_idx >= 0 && led1_idx < (config::NUM_LEDS_TOTAL as i32) {
			let (strip, led) = Racer::_pos2ledstrip(led1_idx);

			colorlists[strip as usize][led as usize].add(&led1_color);
		}

		if led2_idx >= 0 && led2_idx < (config::NUM_LEDS_TOTAL as i32) {
			let (strip, led) = Racer::_pos2ledstrip(led2_idx);

			colorlists[strip as usize][led as usize].add(&led2_color);
		}
	}
}

pub struct Racers
{
	max_energy          : Color,
	min_energy          : Color,
	filtered_energy     : Color,
	filtered_brightness : Color,

	racers_r : Vec<Racer>,
	racers_g : Vec<Racer>,
	racers_b : Vec<Racer>,

	colorlists : [ [Color; config::NUM_LEDS_PER_STRIP]; config::NUM_STRIPS],

	frame_count: usize,

	sigproc: Rc<RefCell<SignalProcessing>>,
}

impl Animation for Racers
{
	fn new(sigproc: Rc<RefCell<SignalProcessing>>) -> Racers
	{
		Racers {
			max_energy: Color{r: 1.0, g: 1.0, b: 1.0, w: 1.0},
			min_energy: Color{r: 0.0, g: 0.0, b: 0.0, w: 0.0},
			filtered_energy: Color{r: 0.0, g: 0.0, b: 0.0, w: 0.0},
			filtered_brightness: Color{r: 0.0, g: 0.0, b: 0.0, w: 0.0},
			racers_r: Vec::with_capacity(NUM_RACERS_R),
			racers_g: Vec::with_capacity(NUM_RACERS_G),
			racers_b: Vec::with_capacity(NUM_RACERS_B),
			colorlists: [ [Color{r: 0.0, g: 0.0, b: 0.0, w: 0.0}; config::NUM_LEDS_PER_STRIP]; config::NUM_STRIPS],
			sigproc: sigproc,
			frame_count: 0,
		}
	}

	fn init(&mut self) -> Result<()>
	{
		let mut rng = rand::thread_rng();

		for _i in 0 .. NUM_RACERS_R {
			let start_pos = rng.gen::<f32>() * (config::NUM_LEDS_TOTAL as f32);
			let speed_scale = 1.0 + SPEED_SCALE_RANGE * (rng.gen::<f32>() - 0.5);
			let mut dir = rng.gen::<i8>();
			if dir > 0 {
				dir = 1;
			} else {
				dir = -1;
			}

			self.racers_r.push(Racer::new(
					RACER_MIN_SPEED_R * speed_scale,
					RACER_MAX_SPEED_R * speed_scale,
					RACER_MIN_BRIGHTNESS_R,
					RACER_MAX_BRIGHTNESS_R,
					Color{r: 1.0, g: 0.0, b: 0.0, w: 0.0},
					start_pos,
					dir));
		}

		for _i in 0 .. NUM_RACERS_G {
			let start_pos = rng.gen::<f32>() * (config::NUM_LEDS_TOTAL as f32);
			let speed_scale = 1.0 + SPEED_SCALE_RANGE * (rng.gen::<f32>() - 0.5);
			let mut dir = rng.gen::<i8>();
			if dir > 0 {
				dir = 1;
			} else {
				dir = -1;
			}

			self.racers_g.push(Racer::new(
					RACER_MIN_SPEED_G * speed_scale,
					RACER_MAX_SPEED_G * speed_scale,
					RACER_MIN_BRIGHTNESS_G,
					RACER_MAX_BRIGHTNESS_G,
					Color{r: 0.0, g: 1.0, b: 0.0, w: 0.0},
					start_pos,
					dir));
		}

		for _i in 0 .. NUM_RACERS_B {
			let start_pos = rng.gen::<f32>() * (config::NUM_LEDS_TOTAL as f32);
			let speed_scale = 1.0 + SPEED_SCALE_RANGE * (rng.gen::<f32>() - 0.5);
			let mut dir = rng.gen::<i8>();
			if dir > 0 {
				dir = 1;
			} else {
				dir = -1;
			}

			self.racers_b.push(Racer::new(
					RACER_MIN_SPEED_B * speed_scale,
					RACER_MAX_SPEED_B * speed_scale,
					RACER_MIN_BRIGHTNESS_B,
					RACER_MAX_BRIGHTNESS_B,
					Color{r: 0.0, g: 0.0, b: 1.0, w: 0.0},
					start_pos,
					dir));
		}

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

		for i in 0..4 {
			let f = self.filtered_energy.ref_by_index_mut(i).unwrap();
			let n = cur_energy.ref_by_index(i).unwrap();

			*f = (1.0 - ENERGY_FILTER_ALPHA) * (*f) + ENERGY_FILTER_ALPHA * (*n);
		}

		// track the maximum energy with cooldown
		self.max_energy.r *= COOLDOWN_FACTOR;
		if self.filtered_energy.r > self.max_energy.r {
			self.max_energy.r = self.filtered_energy.r;
		}

		self.max_energy.g *= COOLDOWN_FACTOR;
		if self.filtered_energy.g > self.max_energy.g {
			self.max_energy.g = self.filtered_energy.g;
		}

		self.max_energy.b *= COOLDOWN_FACTOR;
		if self.filtered_energy.b > self.max_energy.b {
			self.max_energy.b = self.filtered_energy.b;
		}

		self.max_energy.w *= COOLDOWN_FACTOR;
		if self.filtered_energy.w > self.max_energy.w {
			self.max_energy.w = self.filtered_energy.w;
		}

		// track the minimum energy with warmup
		self.min_energy.r += (1.0 - COOLDOWN_FACTOR) * (self.max_energy.r * 0.5 - self.min_energy.r);
		if self.filtered_energy.r < self.min_energy.r {
			self.min_energy.r = self.filtered_energy.r;
		}

		self.min_energy.g += (1.0 - COOLDOWN_FACTOR) * (self.max_energy.g * 0.5 - self.min_energy.g);
		if self.filtered_energy.g < self.min_energy.g {
			self.min_energy.g = self.filtered_energy.g;
		}

		self.min_energy.b += (1.0 - COOLDOWN_FACTOR) * (self.max_energy.b * 0.5 - self.min_energy.b);
		if self.filtered_energy.b < self.min_energy.b {
			self.min_energy.b = self.filtered_energy.b;
		}

		self.min_energy.w += (1.0 - COOLDOWN_FACTOR) * (self.max_energy.w * 0.5 - self.min_energy.w);
		if self.filtered_energy.w < self.min_energy.w {
			self.min_energy.w = self.filtered_energy.w;
		}

		// set all LEDs initially to black
		for strip in 0..config::NUM_STRIPS {
			for led in 0..config::NUM_LEDS_PER_STRIP {
				//self.colorlists[strip][led].scale(FADE_FACTOR);
				self.colorlists[strip][led] = Color{r: 0.0, g: 0.0, b: 0.0, w: 0.0};
			}
		}

		// rescaling and normalization of the energies
		let brightness = Color{
			r: ((self.filtered_energy.r - self.min_energy.r) / (self.max_energy.r - self.min_energy.r)).powf(RGB_EXPONENT),
			g: ((self.filtered_energy.g - self.min_energy.g) / (self.max_energy.g - self.min_energy.g)).powf(RGB_EXPONENT),
			b: ((self.filtered_energy.b - self.min_energy.b) / (self.max_energy.b - self.min_energy.b)).powf(RGB_EXPONENT),
			w: ((self.filtered_energy.w - self.min_energy.w) / (self.max_energy.w - self.min_energy.w)).powf(W_EXPONENT) * W_SCALE,
		};

		// lowpass-filter brightness to reduce intensive fast flashing
		for i in 0..4 {
			let f = self.filtered_brightness.ref_by_index_mut(i).unwrap();
			let n = brightness.ref_by_index(i).unwrap();

			*f = (1.0 - BRIGHTNESS_FILTER_ALPHA) * (*f) + BRIGHTNESS_FILTER_ALPHA * (*n);
		}

		// update all racers
		let f = &self.filtered_brightness;
		let speed = &brightness;
		self.racers_r.iter_mut().for_each(|x| x.update(speed.r, f.r, f.w));
		self.racers_g.iter_mut().for_each(|x| x.update(speed.g, f.g, f.w));
		self.racers_b.iter_mut().for_each(|x| x.update(speed.b, f.b, f.w));

		// render all racers
		for racer in self.racers_r.iter() {
			racer.render(&mut self.colorlists);
		}

		for racer in self.racers_g.iter() {
			racer.render(&mut self.colorlists);
		}

		for racer in self.racers_b.iter() {
			racer.render(&mut self.colorlists);
		}

		// color post-processing
		for strip in 0..config::NUM_STRIPS {
			for led in 0..config::NUM_LEDS_PER_STRIP {
				self.colorlists[strip][led].limit();
			}
		}

		// debug stuff
		self.frame_count += 1;
		if self.frame_count % 100 == 0 {
			println!("---");
			print!("Red   "); dbg_bar(self.min_energy.r, self.filtered_brightness.r, self.max_energy.r); println!("{:11.2}", self.max_energy.r);
			print!("Green "); dbg_bar(self.min_energy.g, self.filtered_brightness.g, self.max_energy.g); println!("{:11.2}", self.max_energy.g);
			print!("Blue  "); dbg_bar(self.min_energy.b, self.filtered_brightness.b, self.max_energy.b); println!("{:11.2}", self.max_energy.b);
			print!("White "); dbg_bar(self.min_energy.w, self.filtered_brightness.w, self.max_energy.w); println!("{:11.2}", self.max_energy.w);
		}

		Ok(())
	}

	fn get_colorlist(&self) -> &[ [Color; config::NUM_LEDS_PER_STRIP]; config::NUM_STRIPS]
	{
		return &self.colorlists;
	}
}
