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

use xcb;
use xkbcommon::xkb;

use error;
use super::Display;

pub struct Keyboard {
	display:   Arc<Display>,
	context:   xkb::Context,
	device:    i32,
	keymap:    xkb::Keymap,
	state:     xkb::State,
	extension: xcb::QueryExtensionData,
}

unsafe impl Send for Keyboard { }
unsafe impl Sync for Keyboard { }

impl Keyboard {
	pub fn new(display: Arc<Display>) -> error::Result<Keyboard> {
		let extension = display.get_extension_data(xcb::xkb::id())
			.ok_or(error::X::MissingExtension)?;

		// Check extension support.
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
				map as u16, map as u16, None).request_check()?
		}

		let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
		let device  = xkb::x11::get_core_keyboard_device_id(&display);
		let keymap  = xkb::x11::keymap_new_from_device(&context, &display, device, xkb::KEYMAP_COMPILE_NO_FLAGS);
		let state   = xkb::x11::state_new_from_device(&keymap, &display, device);
		let mask    = xcb::xkb::EVENT_TYPE_MAP_NOTIFY
			| xcb::xkb::EVENT_TYPE_STATE_NOTIFY
			| xcb::xkb::EVENT_TYPE_NEW_KEYBOARD_NOTIFY;

		xcb::xkb::select_events_checked(&display, device as u16,
			mask as u16, 0, mask as u16, 0, 0, None).request_check()?;

		Ok(Keyboard {
			display:   display,
			context:   context,
			device:    device,
			keymap:    keymap,
			state:     state,
			extension: extension,
		})
	}

	pub fn first_event(&self) -> u8 {
		self.extension.first_event()
	}

	pub fn handle(&mut self, event: &xcb::GenericEvent) {
		match event.response_type() - self.extension.first_event() {
			xcb::xkb::NEW_KEYBOARD_NOTIFY | xcb::xkb::MAP_NOTIFY => {
				self.keymap = xkb::x11::keymap_new_from_device(&self.context, &self.display, self.device, xkb::KEYMAP_COMPILE_NO_FLAGS);
				self.state  = xkb::x11::state_new_from_device(&self.keymap, &self.display, self.device);
			}

			xcb::xkb::STATE_NOTIFY => {
				let event = xcb::cast_event(event): &xcb::xkb::StateNotifyEvent;

				self.state.update_mask(
					event.baseMods() as xkb::ModMask,
					event.latchedMods() as xkb::ModMask,
					event.lockedMods() as xkb::ModMask,
					event.baseGroup() as xkb::LayoutIndex,
					event.latchedGroup() as xkb::LayoutIndex,
					event.lockedGroup() as xkb::LayoutIndex);
			}

			_ => ()
		}
	}

	pub fn symbol(&self, code: xkb::Keycode) -> xkb::Keysym {
		self.state.key_get_one_sym(code)
	}

	pub fn string(&self, code: xkb::Keycode) -> String {
		self.state.key_get_utf8(code)
	}
}
