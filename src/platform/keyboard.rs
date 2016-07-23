use xcb;
use xkbcommon::xkb;

use error;

pub struct Keyboard {
	context:   xkb::Context,
	device:    i32,
	keymap:    xkb::Keymap,
	state:     xkb::State,
	mods:      u8,
	extension: xcb::QueryExtensionData,
}

unsafe impl Send for Keyboard { }
unsafe impl Sync for Keyboard { }

impl Keyboard {
	pub fn new(connection: &xcb::Connection) -> error::Result<Keyboard> {
		let extension = connection.get_extension_data(xcb::xkb::id())
			.ok_or(error::X::MissingExtension)?;

		// Check extension support.
		{
			let cookie = xcb::xkb::use_extension(connection,
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

			xcb::xkb::select_events_checked(connection,
				xcb::xkb::ID_USE_CORE_KBD as u16,
				events as u16, 0, events as u16,
				map as u16, map as u16, None).request_check()?
		}

		let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
		let device  = xkb::x11::get_core_keyboard_device_id(connection);
		let keymap  = xkb::x11::keymap_new_from_device(&context, connection, device, xkb::KEYMAP_COMPILE_NO_FLAGS);
		let state   = xkb::x11::state_new_from_device(&keymap, connection, device);

		xcb::xkb::select_events_checked(connection, device as u16,
			xcb::xkb::EVENT_TYPE_STATE_NOTIFY as u16, 0,
			xcb::xkb::EVENT_TYPE_STATE_NOTIFY as u16, 0, 0, None).request_check()?;

		Ok(Keyboard {
			context:   context,
			device:    device,
			keymap:    keymap,
			state:     state,
			mods:      0,
			extension: extension,
		})
	}

	pub fn first_event(&self) -> u8 {
		self.extension.first_event()
	}

	pub fn update(&mut self, event: &xcb::xkb::StateNotifyEvent) {
		self.state.update_mask(
			event.baseMods() as xkb::ModMask,
			event.latchedMods() as xkb::ModMask,
			event.lockedMods() as xkb::ModMask,
			event.baseGroup() as xkb::LayoutIndex,
			event.latchedGroup() as xkb::LayoutIndex,
			event.lockedGroup() as xkb::LayoutIndex);
	}

	pub fn symbol(&self, code: xkb::Keycode) -> xkb::Keysym {
		self.state.key_get_one_sym(code)
	}

	pub fn string(&self, code: xkb::Keycode) -> String {
		self.state.key_get_utf8(code)
	}
}
