// vim: noet

use std::process::exit;
use std::collections::VecDeque;

use byteorder::{NativeEndian, ReadBytesExt};

use mlua::Lua;

mod signal_processing;
mod config;

use crate::signal_processing::SignalProcessing;

fn main()
{
	let mut stdin = std::io::stdin();

	// test the mlua crate
	let lua_state = Lua::new();

	lua_state.globals().set("get_rust_value", lua_state.create_function(|_, ()| {
		Ok(3)
	}).unwrap()).unwrap();

	let user_script = std::fs::read_to_string("test.lua").unwrap();
	lua_state.load(&user_script).exec().unwrap();

	let lua_func_test : mlua::Function = lua_state.globals().get("test").unwrap();

	println!("{}", lua_func_test.call::<_, u32>(123).unwrap());

	let mut sigproc = SignalProcessing::new(config::BLOCK_LEN, config::SAMP_RATE).unwrap();

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


		sigproc.import_i16_mono_from_iter(samples.iter()).unwrap();
		sigproc.update_fft().unwrap();

		let energy_bass   = sigproc.get_energy_in_band(   0.0,  400.0);
		let energy_mid    = sigproc.get_energy_in_band( 400.0, 4000.0);
		let energy_treble = sigproc.get_energy_in_band(4000.0, config::SAMP_RATE/2.0);

		// dump the output
		println!("Bass: {:8.2} – Mid: {:8.2} – Treble: {:8.2}", energy_bass, energy_mid, energy_treble);
	}

}
