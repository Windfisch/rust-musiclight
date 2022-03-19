// vim: noet

use std::fmt;
use std::error::Error as StdError;

use std::rc::Rc;
use std::cell::RefCell;

use crate::config;
use crate::signal_processing::SignalProcessing;

type Result<T> = std::result::Result<T, AnimationError>;

pub mod particles;
pub mod sparkles;
pub mod racers;

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
