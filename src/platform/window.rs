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

use std::thread;
use std::sync::Arc;
use std::time::Duration;

use xcb;

use error;
use super::{Display, Grab};

pub struct Window {
	display: Arc<Display>,

	id:     u32,
	screen: i32,
	root:   u32,
	cursor: xcb::Cursor,
}

unsafe impl Send for Window { }
unsafe impl Sync for Window { }

impl Window {
	pub fn create(display: Arc<Display>, index: i32, width: u32, height: u32) -> error::Result<Window> {
		let screen = display.get_setup().roots().nth(index as usize).unwrap();
		let id     = display.generate_id();
		let cursor = {
			let pixmap = display.generate_id();
			xcb::create_pixmap(&display, 1, pixmap, screen.root(), 1, 1);

			let cursor = display.generate_id();
			xcb::create_cursor(&display, cursor, pixmap, pixmap, 0, 0, 0, 0, 0, 0, 1, 1);
			xcb::free_pixmap(&display, pixmap);

			cursor
		};

		xcb::create_window(&display, xcb::COPY_FROM_PARENT as u8, id, screen.root(),
			0, 0, width as u16, height as u16,
			0, xcb::WINDOW_CLASS_INPUT_OUTPUT as u16, screen.root_visual(), &[
				(xcb::CW_BACK_PIXEL, screen.black_pixel()),
				(xcb::CW_BACKING_PIXEL, screen.black_pixel()),
				(xcb::CW_BACKING_STORE, xcb::BACKING_STORE_NOT_USEFUL),
				(xcb::CW_CURSOR, cursor),
				(xcb::CW_EVENT_MASK,
					xcb::EVENT_MASK_KEY_PRESS |
					xcb::EVENT_MASK_KEY_RELEASE |
					xcb::EVENT_MASK_BUTTON_PRESS |
					xcb::EVENT_MASK_BUTTON_RELEASE |
					xcb::EVENT_MASK_POINTER_MOTION |
					xcb::EVENT_MASK_EXPOSURE)]);

		display.flush();

		Ok(Window {
			display: display.clone(),

			id:     id,
			screen: index,
			root:   screen.root(),
			cursor: cursor,
		})
	}

	pub fn flush(&self) {
		self.display.flush();
	}

	pub fn id(&self) -> u32 {
		self.id
	}

	pub fn screen(&self) -> i32 {
		self.screen
	}

	pub fn root(&self) -> u32 {
		self.root
	}

	/// Resize the window.
	pub fn resize(&self, width: u32, height: u32) {
		xcb::configure_window(&self.display, self.id(), &[
			(xcb::CONFIG_WINDOW_WIDTH as u16, width),
			(xcb::CONFIG_WINDOW_HEIGHT as u16, height)]);

		self.flush();
	}

	pub fn dimensions(&self) -> (u32, u32) {
		if let Ok(reply) = xcb::get_geometry(&self.display, self.id()).get_reply() {
			(reply.width() as u32, reply.height() as u32)
		}
		else {
			(0, 0)
		}
	}

	/// Grab the given input.
	pub fn grab(&self, grab: Grab) -> error::Result<()> {
		let result = match grab {
			Grab::Keyboard => {
				xcb::grab_keyboard(&self.display, false, self.id(), xcb::CURRENT_TIME,
					xcb::GRAB_MODE_ASYNC as u8, xcb::GRAB_MODE_ASYNC as u8
				).get_reply()?.status()
			}

			Grab::Pointer => {
				xcb::grab_pointer(&self.display, false, self.id(),
					(xcb::EVENT_MASK_BUTTON_PRESS | xcb::EVENT_MASK_BUTTON_RELEASE | xcb::EVENT_MASK_POINTER_MOTION) as u16,
					xcb::GRAB_MODE_ASYNC as u8, xcb::GRAB_MODE_ASYNC as u8, 0, self.cursor, xcb::CURRENT_TIME
				).get_reply()?.status()
			}
		};

		match result as u32 {
			xcb::GRAB_STATUS_SUCCESS =>
				Ok(()),

			xcb::GRAB_STATUS_ALREADY_GRABBED =>
				Err(error::Grab::Conflict.into()),

			xcb::GRAB_STATUS_NOT_VIEWABLE =>
				Err(error::Grab::Unmapped.into()),

			xcb::GRAB_STATUS_FROZEN =>
				Err(error::Grab::Frozen.into()),

			_ =>
				unreachable!()
		}
	}

	/// Try to grab the given input with 1ms pauses.
	pub fn try_grab(&self, grab: Grab, tries: usize) -> error::Result<()> {
		let mut result = Ok(());

		for _ in 0 .. tries {
			result = self.grab(grab);

			if result.is_ok() {
				break;
			}

			thread::sleep(Duration::from_millis(1));
		}

		result
	}
}

impl Drop for Window {
	fn drop(&mut self) {
		xcb::destroy_window(&self.display, self.id);
	}
}
