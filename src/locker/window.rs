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
use std::ops::Deref;

use xcb;

use error;
use super::Display;
use platform;

pub struct Window {
	display: Arc<Display>,
	window:  platform::Window,
	gc:      u32,
	cursor:  u32,

	locked:   bool,
	keyboard: bool,
	pointer:  bool,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Grab {
	Keyboard,
	Pointer,
}

impl Window {
	pub fn create(display: Arc<Display>, index: i32) -> error::Result<Window> {
		let screen = display.get_setup().roots().nth(display.screen() as usize).unwrap();
		let window = platform::Window::create((**display).clone(), display.screen(),
			screen.width_in_pixels() as u32, screen.height_in_pixels() as u32)?;

		let cursor = {
			let pixmap = display.generate_id();
			xcb::create_pixmap(&display, 1, pixmap, screen.root(), 1, 1);

			let cursor = display.generate_id();
			xcb::create_cursor(&display, cursor, pixmap, pixmap, 0, 0, 0, 0, 0, 0, 1, 1);
			xcb::free_pixmap(&display, pixmap);

			cursor
		};

		xcb::change_window_attributes(&display, window.id(), &[
			(xcb::CW_CURSOR, cursor),
			(xcb::CW_OVERRIDE_REDIRECT, 1)]);

		xcb::change_property(&display, xcb::PROP_MODE_REPLACE as u8, window.id(),
			xcb::intern_atom(&display, false, "SCREENRUSTER_SAVER").get_reply()?.atom(),
			xcb::ATOM_CARDINAL, 32, &[index]);

		let gc = display.generate_id();
		xcb::create_gc(&display, gc, window.id(), &[(xcb::GC_FOREGROUND, screen.black_pixel())]);

		display.flush();

		Ok(Window {
			display: display.clone(),
			window:  window,
			gc:      gc,
			cursor:  cursor,

			locked:   false,
			keyboard: false,
			pointer:  false,
		})
	}

	/// Check if the window is locked.
	pub fn is_locked(&self) -> bool {
		self.locked
	}

	/// Check if the window grabbed the keyboard.
	pub fn has_keyboard(&self) -> bool {
		self.keyboard
	}

	/// Check if the window grabbed the pointer.
	pub fn has_pointer(&self) -> bool {
		self.pointer
	}

	/// Sanitize the window.
	pub fn sanitize(&mut self) {
		if self.locked {
			// Try to grab the keyboard again in case it wasn't grabbed when locking.
			if !self.keyboard && self.grab(Grab::Keyboard).is_ok() {
				self.keyboard = true;
			}

			// Try to grab the pointer again in case it wasn't grabbed when locking.
			if !self.pointer && self.grab(Grab::Pointer).is_ok() {
				self.pointer = true;
			}

			// Remap the window in case stuff like popups went above the locker.
			xcb::map_window(&self.display, self.id());
			xcb::configure_window(&self.display, self.id(), &[
				(xcb::CONFIG_WINDOW_STACK_MODE as u16, xcb::STACK_MODE_ABOVE)]);
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

	/// Lock the window.
	pub fn lock(&mut self) -> error::Result<()> {
		if self.locked {
			return Ok(());
		}

		// Map the window and make sure it's raised.
		xcb::map_window(&self.display, self.id());
		xcb::configure_window(&self.display, self.id(), &[
			(xcb::CONFIG_WINDOW_STACK_MODE as u16, xcb::STACK_MODE_ABOVE)]);

		// Try to grab the keyboard and mouse.
		self.keyboard = self.try_grab(Grab::Keyboard, 500).is_ok();
		self.pointer  = self.try_grab(Grab::Pointer, 500).is_ok();

		// Some retarded X11 applications grab the keyboard and pointer for long
		// periods of time for no reason, so try to change focus and grab again.
		if !self.keyboard || !self.pointer {
			warn!("could not grab keyboard or pointer, trying to change focus");

			xcb::set_input_focus(&self.display, xcb::INPUT_FOCUS_POINTER_ROOT as u8, self.id(), xcb::CURRENT_TIME);
			self.flush();

			// Failing to grab the keyboard is fatal since the window manager or
			// other applications may be stealing our thunder.
			if !self.keyboard {
				if let Err(err) = self.try_grab(Grab::Keyboard, 500) {
					warn!("could not grab pointer: {:?}", err);
				}
				else {
					self.keyboard = true;
				}
			}

			if !self.pointer {
				if let Err(err) = self.try_grab(Grab::Pointer, 500) {
					warn!("could not grab pointer: {:?}", err);
				}
				else {
					self.pointer = true;
				}
			}
		}

		// Listen for window change events.
		xcb::change_window_attributes(&self.display, self.root(), &[
			(xcb::CW_EVENT_MASK, xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY)]);

		// If the display supports XRandr listen for screen change events.
		if self.display.randr().is_some() {
			xcb::randr::select_input(&self.display, self.id(),
				xcb::randr::NOTIFY_MASK_SCREEN_CHANGE as u16);
		}

		self.locked = true;

		Ok(())
	}

	/// Notify the window the power status changed.
	pub fn power(&mut self, value: bool) {
		if !value {
			xcb::change_window_attributes(&self.display, self.id(), &[
				(xcb::CW_BACK_PIXEL, self.black())]);
		}
	}

	/// Make the window solid black.
	pub fn blank(&mut self) {
		let (width, height) = self.dimensions();

		xcb::poly_fill_rectangle(&self.display, self.id(), self.gc, &[
			xcb::Rectangle::new(0, 0, width as u16, height as u16)]);

		self.flush();
	}

	/// Unlock the window, hiding and ungrabbing whatever.
	pub fn unlock(&mut self) -> error::Result<()> {
		if !self.locked {
			return Ok(());
		}

		xcb::ungrab_keyboard(&self.display, xcb::CURRENT_TIME);
		self.keyboard = false;

		xcb::ungrab_pointer(&self.display, xcb::CURRENT_TIME);
		self.pointer = false;

		xcb::unmap_window(&self.display, self.id());
		self.locked = false;

		self.flush();

		Ok(())
	}
}

impl Deref for Window {
	type Target = platform::Window;

	fn deref(&self) -> &Self::Target {
		&self.window
	}
}
