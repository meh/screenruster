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
use std::ops::Deref;

use xcb;

use error;
use config;
use platform;

pub struct Display {
	display: Arc<platform::Display>,

	randr: bool,
	dpms:  bool,
}

unsafe impl Send for Display { }
unsafe impl Sync for Display { }

impl Display {
	/// Open the display.
	pub fn open(config: config::Locker) -> error::Result<Arc<Display>> {
		let     display = platform::Display::open(config.display())?;
		let     randr   = display.get_extension_data(xcb::randr::id());
		let mut dpms    = display.get_extension_data(xcb::dpms::id());

		if randr.is_some() {
			let cookie = xcb::randr::query_version(&display, 1, 1);
			let reply  = cookie.get_reply()?;

			if reply.major_version() < 1 || (reply.major_version() >= 1 && reply.minor_version() < 1) {
				return Err(error::X::MissingExtension.into());
			}
		}

		if let Some(ext) = dpms.take() {
			if config.dpms() && xcb::dpms::capable(&display).get_reply()?.capable() {
				dpms = Some(ext);
			}
		}

		let display = Arc::new(Display {
			display: display,

			randr: randr.is_some(),
			dpms:  dpms.is_some(),
		});

		display.sanitize();

		Ok(display)
	}

	/// Get the XRandr extension data.
	pub fn randr(&self) -> Option<xcb::QueryExtensionData> {
		if self.randr {
			Some(self.display.get_extension_data(xcb::randr::id()).unwrap())
		}
		else {
			None
		}
	}

	/// Get the DPMS extension data.
	pub fn dpms(&self) -> Option<xcb::QueryExtensionData> {
		if self.dpms {
			Some(self.display.get_extension_data(xcb::dpms::id()).unwrap())
		}
		else {
			None
		}
	}

	/// Check if the monitor is powered on or not.
	pub fn is_powered(&self) -> bool {
		if !self.dpms {
			return true;
		}

		if let Ok(reply) = xcb::dpms::info(self).get_reply() {
			if !reply.state() {
				return true;
			}

			match reply.power_level() as u32 {
				xcb::dpms::DPMS_MODE_ON =>
					true,

				xcb::dpms::DPMS_MODE_OFF |
				xcb::dpms::DPMS_MODE_STANDBY |
				xcb::dpms::DPMS_MODE_SUSPEND =>
					false,

				_ => unreachable!()
			}
		}
		else {
			false
		}
	}

	/// Turn the monitor on or off.
	pub fn power(&self, value: bool) {
		if !self.dpms || self.is_powered() == value {
			return;
		}

		xcb::dpms::force_level(self, if value {
			xcb::dpms::DPMS_MODE_ON
		} else {
			xcb::dpms::DPMS_MODE_OFF
		} as u16);

		self.flush();
	}

	/// Sanitize the display from bad X11 things.
	pub fn sanitize(&self) {
		// Reset DPMS settings to usable.
		if self.dpms {
			xcb::dpms::set_timeouts(self, 0, 0, 0);
			xcb::dpms::enable(self);
		}

		// Reset screen saver timeout.
		xcb::set_screen_saver(self, 0, 0, 0, xcb::EXPOSURES_ALLOWED as u8);
	}

	/// Observe events on the given window and all its children.
	pub fn observe(&self, window: u32) {
		macro_rules! try {
			($body:expr) => (
				if let Ok(value) = $body {
					value
				}
				else {
					return;
				}
			);
		}

		let query = try!(xcb::query_tree(self, window).get_reply());

		// Return if the window is one of ours.
		{
			let reply = xcb::get_property(self, false, window,
				xcb::intern_atom(self, false, "SCREENRUSTER_SAVER").get_reply().unwrap().atom(),
				xcb::ATOM_CARDINAL, 0, 1).get_reply();

			if let Ok(reply) = reply {
				if reply.type_() == xcb::ATOM_CARDINAL {
					return;
				}
			}
		}

		// Start listening for activity events from the window making sure to not
		// break it, by excluding various events.
		let attrs = try!(xcb::get_window_attributes(self, window).get_reply());
		try!(xcb::change_window_attributes_checked(self, window, &[
			(xcb::CW_EVENT_MASK, (attrs.all_event_masks() | attrs.do_not_propagate_mask() as u32) &
				(xcb::EVENT_MASK_KEY_PRESS | xcb::EVENT_MASK_KEY_RELEASE) |
				(xcb::EVENT_MASK_POINTER_MOTION | xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY))]).request_check());

		for &child in query.children() {
			self.observe(child);
		}
	}
}

impl Deref for Display {
	type Target = Arc<platform::Display>;

	fn deref(&self) -> &Self::Target {
		&self.display
	}
}
