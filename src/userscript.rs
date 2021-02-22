// vim: noet

/*
 * Module for the Lua scripting interface of Musiclight.
 */

/* Test code for reference
	// test the mlua crate
	let lua_state = Lua::new();

	lua_state.globals().set("get_rust_value", lua_state.create_function(|_, ()| {
		Ok(3)
	}).unwrap()).unwrap();

	let user_script = std::fs::read_to_string("test.lua").unwrap();
	lua_state.load(&user_script).exec().unwrap();

	let lua_func_test : mlua::Function = lua_state.globals().get("test").unwrap();

	println!("{}", lua_func_test.call::<_, u32>(123).unwrap());
*/

use crate::config;
use crate::signal_processing::SignalProcessing;

use mlua::Lua;
use mlua::FromLua;
use mlua::Error;

use std::rc::Rc;
use std::cell::RefCell;

pub struct UserScript
{
	lua_state: Lua,

	pub colorlists: [ [f32; config::NUM_LEDS_TOTAL]; 4],
}

impl UserScript
{
	pub fn new(sigproc: Rc<RefCell<SignalProcessing>>, user_script_path: &str) -> std::result::Result<UserScript, mlua::Error>
	{
		let s = UserScript {
			lua_state: Lua::new(),
			colorlists: [ [0f32; config::NUM_LEDS_TOTAL]; 4],
		};

		// provide some configuration constants to Lua via a table
		let config_table = s.lua_state.create_table()?;

		config_table.set("sampling_rate", config::SAMP_RATE)?;
		config_table.set("block_length", config::BLOCK_LEN)?;
		config_table.set("samples_per_update", config::SAMPLES_PER_UPDATE)?;

		s.lua_state.globals().set("CONFIG", config_table)?;

		// register the signal processing reference as Lua user data
		s.lua_state.globals().set("sigproc", SignalProcessingWrapper{
			signal_processing: sigproc
		})?;

		// load the user script and execute it to make variables and functions available
		let user_script = std::fs::read_to_string(user_script_path)?;
		s.lua_state.load(&user_script).exec()?;

		Ok(s)
	}

	pub fn init(&self) -> std::result::Result<(), mlua::Error>
	{
		// find the init functions
		let lua_init_func: mlua::Function = self.lua_state.globals().get("init")?;

		lua_init_func.call( (config::NUM_STRIPS, config::NUM_LEDS_PER_STRIP) )?;

		Ok(())
	}

	pub fn periodic(&mut self) -> std::result::Result<(), mlua::Error>
	{
		// find the init functions
		let lua_periodic_func: mlua::Function = self.lua_state.globals().get("periodic")?;

		// call the script's periodic() function, which (hopefully) returns four Tables with color
		// values
		let rvals = lua_periodic_func.call::<_, mlua::MultiValue>( () )?;

		// check the number of returned values
		if rvals.len() != 4 {
			return Err(Error::RuntimeError("Wrong number of return values from 'periodic'. Expected 4.".to_string()));
		}

		// convert the Lua Tables to normal vectors
		let mut i = 0;
		for rval in rvals {
			let table = mlua::Table::from_lua(rval, &self.lua_state)?;

			let v = table.sequence_values()
			             .map(|x| x.unwrap())
			             .collect::<Vec<f32>>();

			// check the length of the color array
			if v.len() != config::NUM_LEDS_TOTAL {
				return Err(Error::RuntimeError("Number of color values returned from 'periodic' must match number of LEDs given in 'init'.".to_string()));
			}

			for j in 0 .. self.colorlists[i].len() {
				self.colorlists[i][j] = v[j];
			}

			i += 1;
		}

		Ok(())
	}
}


/*
 * Wrap a SignalProcessing instance and provide a Lua interface for some of its methods.
 */
struct SignalProcessingWrapper
{
	signal_processing: Rc<RefCell<SignalProcessing>>,
}

impl mlua::UserData for SignalProcessingWrapper
{
	fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M)
	{
		methods.add_method("get_energy_in_band", |_, this, (start_freq, end_freq): (f32, f32)| {
			Ok(this.signal_processing.borrow().get_energy_in_band(start_freq, end_freq))
		});
	}
}

