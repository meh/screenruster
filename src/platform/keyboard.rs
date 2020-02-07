// Copyleft (ↄ) meh. <meh@schizofreni.co> | http://meh.schizofreni.co
//
// This file is part of screenruster.
//
// screenruster is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// screenruster is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with screenruster.  If not, see <http://www.gnu.org/licenses/>.

use std::sync::Arc;
use std::env;

use xcb;
use xkb;

use crate::error;
use super::Display;

/// Keyboard manager and handler.
///
/// Its job is to map X key events to proper symbols/strings based on the
/// layout and mappings.
pub struct Keyboard {
	display: Arc<Display>,
	context: xkb::Context,
	device:  i32,
	keymap:  xkb::Keymap,
	state:   xkb::State,
	#[allow(dead_code)]
	table:   xkb::compose::Table,
	compose: xkb::compose::State,
}

unsafe impl Send for Keyboard { }
unsafe impl Sync for Keyboard { }

impl Keyboard {
	/// Create a keyboard for the given display.
	pub fn new(display: Arc<Display>, locale: Option<&str>) -> error::Result<Keyboard> {
		display.get_extension_data(xcb::xkb::id())
			.ok_or(error::X::MissingExtension)?;

		// Check the XKB extension version.
		{
			let cookie = xcb::xkb::use_extension(&display,
				xkb::x11::MIN_MAJOR_XKB_VERSION,
				xkb::x11::MIN_MINOR_XKB_VERSION);

			if !cookie.get_reply()?.supported() {
				return Err(error::X::MissingExtension.into());
			}
		}

		// Select events.
		{
			let map =
				xcb::xkb::MAP_PART_KEY_TYPES |
				xcb::xkb::MAP_PART_KEY_SYMS |
				xcb::xkb::MAP_PART_MODIFIER_MAP |
				xcb::xkb::MAP_PART_EXPLICIT_COMPONENTS |
				xcb::xkb::MAP_PART_KEY_ACTIONS |
				xcb::xkb::MAP_PART_KEY_BEHAVIORS |
				xcb::xkb::MAP_PART_VIRTUAL_MODS |
				xcb::xkb::MAP_PART_VIRTUAL_MOD_MAP;

			let events =
				xcb::xkb::EVENT_TYPE_NEW_KEYBOARD_NOTIFY |
				xcb::xkb::EVENT_TYPE_MAP_NOTIFY |
				xcb::xkb::EVENT_TYPE_STATE_NOTIFY;

			xcb::xkb::select_events_checked(&display,
				xcb::xkb::ID_USE_CORE_KBD as u16,
				events as u16, 0, events as u16,
				map as u16, map as u16, None).request_check()?;
		}

		let context = xkb::Context::default();
		let device  = xkb::x11::device(&display)?;
		let keymap  = xkb::x11::keymap(&display, device, &context, Default::default())?;
		let state   = xkb::x11::state(&display, device, &keymap)?;

		let (table, compose) = {
			let locale = locale.map(String::from).or(env::var("LANG").ok()).unwrap_or("C".into());
			let table  = if let Ok(table) = xkb::compose::Table::new(&context, &locale, Default::default()) {
				table
			}
			else {
				xkb::compose::Table::new(&context, "C", Default::default()).unwrap()
			};

			let state = table.state(Default::default());

			(table, state)
		};

		Ok(Keyboard { display, context, device, keymap, state, table, compose })
	}

	/// Get the extension data.
	pub fn extension(&self) -> xcb::QueryExtensionData {
		self.display.get_extension_data(xcb::xkb::id()).unwrap()
	}

	/// Checks if an event belongs to the keyboard.
	pub fn owns_event(&self, event: u8) -> bool {
		event >= self.extension().first_event() &&
		event < self.extension().first_event() + xcb::xkb::EXTENSION_DEVICE_NOTIFY
	}

	/// Handles an X event.
	pub fn handle(&mut self, event: &xcb::GenericEvent) {
		match event.response_type() - self.extension().first_event() {
			xcb::xkb::NEW_KEYBOARD_NOTIFY | xcb::xkb::MAP_NOTIFY => {
				self.keymap = xkb::x11::keymap(&self.display, self.device, &self.context, Default::default()).unwrap();
				self.state  = xkb::x11::state(&self.display, self.device, &self.keymap).unwrap();
			}

			xcb::xkb::STATE_NOTIFY => {
				let event = unsafe { xcb::cast_event::<xcb::xkb::StateNotifyEvent>(event) };

				self.state.update().mask(
					event.base_mods(),
					event.latched_mods(),
					event.locked_mods(),
					event.base_group(),
					event.latched_group(),
					event.locked_group());
			}

			_ => ()
		}
	}

	/// Translate a key code to the key symbol.
	pub fn symbol(&self, code: u8) -> Option<xkb::Keysym> {
		self.state.key(code).sym()
	}

	/// Translate a key code to an UTF-8 string.
	pub fn string(&self, code: u8) -> Option<String> {
		self.state.key(code).utf8()
	}
}
