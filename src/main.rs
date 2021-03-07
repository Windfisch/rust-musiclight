// vim: noet

use std::process::exit;
use std::collections::VecDeque;

use byteorder::{NativeEndian, ReadBytesExt};

mod signal_processing;
mod config;
mod userscript;
mod udpproto;
mod animation;

use crate::signal_processing::SignalProcessing;
//use crate::userscript::UserScript;
use crate::udpproto::UdpProto;
use crate::animation::Animation;

use std::rc::Rc;
use std::cell::RefCell;

use std::thread::sleep;
use std::time::{Duration, Instant};

fn main()
{
	let mut stdin = std::io::stdin();

	// set up the UDP protocol
	let mut udpproto = match UdpProto::new(config::UDP_SERVER_ADDR) {
		Ok(u) => u,
		Err(e) => {
			println!("Error during UDP client setup:\n{}", e);
			exit(1);
		}
	};

	// set up signal processing

	println!("Initializing signal processing...");

	let sigproc = Rc::new(RefCell::new(
	                  SignalProcessing::new(config::BLOCK_LEN, config::SAMP_RATE).unwrap()));

	/*
	// set up Lua environment

	println!("Loading user script...");

	let mut script = match UserScript::new(sigproc.clone(), "particles.lua") {
		Ok(script) => script,
		Err(e) => {
			println!("=== Lua Error ===\n{}\n====> Terminating.", e);
			exit(1);
		}
	};

	println!("Calling init()...");

	match script.init() {
		Ok(_) => (),
		Err(e) => {
			println!("=== Lua Error ===\n{}\n====> Terminating.", e);
			exit(1);
		}
	};
	*/

	println!("Contructing Animation...");

	let mut anim: animation::particles::Particles = animation::Animation::new(sigproc.clone());

	println!("Calling Animation::init()...");

	anim.init().unwrap();

	println!("Done! Starting main loop…");

	// Timing setup

	let block_period = Duration::from_nanos((0.95 * (config::SAMPLES_PER_UPDATE as f32) * 1e9 / config::SAMP_RATE) as u64);
	let send_period = Duration::from_nanos(1000000000 / 60);

	let mut next_block_instant = Instant::now() + block_period;
	let mut next_send_instant = Instant::now() + send_period;

	// array for samples directly read from stream
	let mut samples: VecDeque<i16> = VecDeque::with_capacity(config::BLOCK_LEN);

	// main loop
	loop {

		// read a block of samples and exit gracefully on EOF
		for _i in 0 .. config::SAMPLES_PER_UPDATE {
			// avoid increasing the size of the deque
			if samples.len() == config::BLOCK_LEN {
				samples.pop_front();
			}

			// read a sample from the input
			let res = stdin.read_i16::<NativeEndian>();

			// if everything is ok, append it to the samples deque
			match res {
				Ok(s) => samples.push_back(s),
				Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
					println!("End of stream. Exiting.");
					exit(0);
				},
				Err(e) => panic!(e)
			}
		}

		// only run calculations if the deque has been filled enough
		if samples.len() < config::BLOCK_LEN {
			continue;
		}

		// run the signal processing
		{
			let mut s = sigproc.borrow_mut();
			s.import_i16_mono_from_iter(samples.iter()).unwrap();
			s.update_fft().unwrap();
		}

		// call the periodic function in the user script
		match anim.periodic() {
			Ok(_) => (),
			Err(e) => {
				println!("=== Animation Error ===\n{}\n====> Terminating.", e);
				exit(1);
			}
		};

		if Instant::now() > next_send_instant {
			let colorlists = anim.get_colorlist();

			for i in 0..config::NUM_LEDS_TOTAL {
				let strip = i / config::NUM_LEDS_PER_STRIP;
				let led   = i % config::NUM_LEDS_PER_STRIP;

				udpproto.set_color(strip as u8,
								   led as u8,
								   (colorlists[strip][led].r * 255.0) as u8,
								   (colorlists[strip][led].g * 255.0) as u8,
								   (colorlists[strip][led].b * 255.0) as u8,
								   (colorlists[strip][led].w * 255.0) as u8).unwrap();
			}

			udpproto.commit().unwrap();

			next_send_instant += send_period;
		}

		let now = Instant::now();
		if now < next_block_instant {
			sleep(next_block_instant - now);
		}

		next_block_instant = Instant::now() + block_period;
	}

}
