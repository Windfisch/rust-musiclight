// vim: noet

use std::process::exit;
use std::collections::VecDeque;

use byteorder::{NativeEndian, ReadBytesExt};

mod signal_processing;
mod config;
mod userscript;

use crate::signal_processing::SignalProcessing;
use crate::userscript::UserScript;

use std::rc::Rc;
use std::cell::RefCell;

fn main()
{
	let mut stdin = std::io::stdin();

	// set up signal processing

	println!("Initializing signal processing...");

	let sigproc = Rc::new(RefCell::new(
	                  SignalProcessing::new(config::BLOCK_LEN, config::SAMP_RATE).unwrap()));

	// set up Lua environment

	println!("Loading user script...");

	let script = match UserScript::new(sigproc.clone(), "test.lua") {
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

	println!("Done! Starting main loop…");

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
		match script.periodic() {
			Ok(_) => (),
			Err(e) => {
				println!("=== Lua Error ===\n{}\n====> Terminating.", e);
				exit(1);
			}
		};

	}

}
