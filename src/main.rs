// vim: noet

use std::process::exit;
use std::collections::VecDeque;

use byteorder::{NativeEndian, ReadBytesExt};

mod signal_processing;
mod config;
mod udpproto;
mod animation;

use crate::signal_processing::SignalProcessing;
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
	let mut udpproto = match UdpProto::new(config::UDP_SERVER_ADDR, config::NUM_LEDS_TOTAL) {
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

	println!("Contructing Animation...");

	// TODO: let the user select via the command line
	//let mut anim: animation::particles::Particles = animation::Animation::new(sigproc.clone());
	//let mut anim: animation::sparkles::Sparkles = animation::Animation::new(sigproc.clone());
	let mut anim: animation::racers::Racers = animation::Animation::new(sigproc.clone());
	//let mut anim: animation::spectrum::Spectrum = animation::Animation::new(sigproc.clone());

	println!("Calling Animation::init()...");

	anim.init().unwrap();

	println!("Done! Starting main loop…");

	// Timing setup

	let block_period = Duration::from_nanos((0.95 * (config::SAMPLES_PER_UPDATE as f32) * 1e9 / config::SAMP_RATE) as u64);
	let send_period = Duration::from_nanos((1000000000.0 / config::FPS_LEDS) as u64);

	let max_lag = 5*send_period;

	let mut next_block_instant = Instant::now() + block_period;
	let mut next_send_instant = Instant::now() + send_period;

	// array for samples directly read from stream
	let mut samples: VecDeque<i16> = VecDeque::with_capacity(config::BLOCK_LEN);

	// counts silent (zero-valued) samples
	let mut silent_samples: usize = 0;

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

			if s.is_silent() {
				silent_samples += config::BLOCK_LEN;

				if silent_samples >= config::STANDBY_MAX_SILENT_SAMPLES {
					// too many silent samples in a row: stop any signal processing until something
					// else occurs at the input again
					continue;
				}
			} else {
				silent_samples = 0;
			}

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

				let r = udpproto.set_color(strip as u8,
				                           led,
				                           (colorlists[strip][led].r * 255.0) as u8,
				                           (colorlists[strip][led].g * 255.0) as u8,
				                           (colorlists[strip][led].b * 255.0) as u8,
				                           (colorlists[strip][led].w * 255.0) as u8);

				match r {
					Ok(_) => (),
					Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
						// try again in one second
						next_send_instant += Duration::from_secs(1);
						break;
					}
					Err(e) => panic!(e),
				}
			}

			match udpproto.commit() {
					Ok(_) => (),
					Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
						// try again in one second
						next_send_instant += Duration::from_secs(1);
					}
					Err(e) => panic!(e),
			}

			let now = Instant::now();
			if now > (next_send_instant + max_lag) {
				println!("Warning! Lag exceeds {:?}. Resetting sender timing.", max_lag);
				next_send_instant = now + send_period;
			} else {
				next_send_instant += send_period;
			}
		}

		let now = Instant::now();
		if now < next_block_instant {
			sleep(next_block_instant - now);
		}

		next_block_instant = Instant::now() + block_period;
	}

}
