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
use xcb_util::icccm;

use error;
use platform::{self, Display};

pub struct Window {
	display: Arc<Display>,
	window:  platform::Window,
}

impl Window {
	pub fn create(display: Arc<Display>) -> error::Result<Window> {
		let screen = display.get_setup().roots().nth(display.screen() as usize).unwrap();
		let window = platform::Window::create(display.clone(), display.screen(),
			(screen.width_in_pixels() as f32 / 1.2) as u32,
			(screen.height_in_pixels() as f32 / 1.2) as u32)?;

		xcb::change_property(&display, xcb::PROP_MODE_REPLACE as u8, window.id(),
			xcb::ATOM_WM_NAME, xcb::ATOM_STRING, 8, b"ScreenRuster");

		icccm::set_wm_size_hints(&display, window.id(), xcb::ATOM_WM_NORMAL_HINTS, &icccm::SizeHints::empty()
			.aspect((screen.width_in_pixels() as i32, screen.height_in_pixels() as i32),
			        (screen.width_in_pixels() as i32, screen.height_in_pixels() as i32))
			.build());

		display.flush();

		Ok(Window {
			display: display.clone(),
			window:  window,
		})
	}

	pub fn show(&self) {
		xcb::map_window(&self.display, self.id());

		self.flush();
	}
}

impl Deref for Window {
	type Target = platform::Window;

	fn deref(&self) -> &Self::Target {
		&self.window
	}
}
