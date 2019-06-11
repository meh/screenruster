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

use crate::error;
use super::Display;

pub struct Window {
	display: Arc<Display>,

	id:     u32,
	screen: i32,
	root:   u32,
	black:  u32,
}

unsafe impl Send for Window { }
unsafe impl Sync for Window { }

impl Window {
	pub fn create(display: Arc<Display>, index: i32, width: u32, height: u32) -> error::Result<Window> {
		let screen = display.get_setup().roots().nth(index as usize).unwrap();
		let id     = display.generate_id();

		xcb::create_window(&display, xcb::COPY_FROM_PARENT as u8, id, screen.root(),
			0, 0, width as u16, height as u16,
			0, xcb::WINDOW_CLASS_INPUT_OUTPUT as u16, screen.root_visual(), &[
				(xcb::CW_BORDER_PIXEL, screen.black_pixel()),
				(xcb::CW_BACKING_PIXEL, screen.black_pixel()),
				(xcb::CW_BACKING_STORE, xcb::BACKING_STORE_NOT_USEFUL),
				(xcb::CW_EVENT_MASK,
					xcb::EVENT_MASK_KEY_PRESS |
					xcb::EVENT_MASK_KEY_RELEASE |
					xcb::EVENT_MASK_BUTTON_PRESS |
					xcb::EVENT_MASK_BUTTON_RELEASE |
					xcb::EVENT_MASK_POINTER_MOTION |
					xcb::EVENT_MASK_STRUCTURE_NOTIFY |
					xcb::EVENT_MASK_EXPOSURE)]);

		display.flush();

		Ok(Window {
			display: display.clone(),

			id:     id,
			screen: index,
			root:   screen.root(),
			black:  screen.black_pixel(),
		})
	}

	/// Flush the request queue.
	pub fn flush(&self) {
		self.display.flush();
	}

	/// Get the id.
	pub fn id(&self) -> u32 {
		self.id
	}

	/// Get the screen.
	pub fn screen(&self) -> i32 {
		self.screen
	}

	/// Get the screen root.
	pub fn root(&self) -> u32 {
		self.root
	}

	/// Get the black pixel.
	pub fn black(&self) -> u32 {
		self.black
	}

	/// Resize the window.
	pub fn resize(&self, width: u32, height: u32) {
		xcb::configure_window(&self.display, self.id(), &[
			(xcb::CONFIG_WINDOW_WIDTH as u16, width),
			(xcb::CONFIG_WINDOW_HEIGHT as u16, height)]);

		self.flush();
	}

	/// Get the dimensions.
	pub fn dimensions(&self) -> (u32, u32) {
		if let Ok(reply) = xcb::get_geometry(&self.display, self.id()).get_reply() {
			(reply.width() as u32, reply.height() as u32)
		}
		else {
			(0, 0)
		}
	}
}

impl Drop for Window {
	fn drop(&mut self) {
		xcb::destroy_window(&self.display, self.id);
	}
}
