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

use mlua::Lua;
use mlua::FromLua;
use mlua::Error;

pub struct UserScript
{
	lua_state: Lua,
}

impl UserScript
{
	pub fn new(user_script_path: &str) -> std::result::Result<UserScript, mlua::Error>
	{
		let s = UserScript {
			lua_state: Lua::new(),
		};

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

	pub fn periodic(&self) -> std::result::Result<(), mlua::Error>
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
		let mut colorlists = Vec::<Vec<f32>>::new();

		for rval in rvals {
			let table = mlua::Table::from_lua(rval, &self.lua_state)?;

			let v = table.sequence_values()
			             .map(|x| x.unwrap())
			             .collect::<Vec<f32>>();

			// check the length of the color array
			if v.len() != config::NUM_LEDS_TOTAL {
				return Err(Error::RuntimeError("Number of color values returned from 'periodic' must match number of LEDs given in 'init'.".to_string()));
			}

			colorlists.push(v);
		}

		println!("{:?}", colorlists[0][0]);

		Ok(())
	}
}
