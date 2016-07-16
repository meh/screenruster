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

use std::ptr;
use std::mem;
use std::time::Duration;
use std::thread;
use std::sync::Arc;

use libc::{c_int, c_uint};
use x11::{xlib, glx, xrandr};

use error;
use util;
use super::Display;

#[derive(Debug)]
pub struct Window {
	pub id:     xlib::Window,
	pub width:  u32,
	pub height: u32,

	pub display: Arc<Display>,
	pub screen:  c_int,
	pub root:    xlib::Window,
	pub cursor:  xlib::Cursor,
	pub im:      xlib::XIM,
	pub ic:      xlib::XIC,

	pub locked:   bool,
	pub keyboard: bool,
	pub pointer:  bool,
}

unsafe impl Send for Window { }
unsafe impl Sync for Window { }

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Grab {
	Keyboard,
	Pointer,
}

impl Window {
	pub fn create(display: Arc<Display>, screen: c_int) -> error::Result<Window> {
		unsafe {
			let root   = xlib::XRootWindow(display.id, screen);
			let width  = xlib::XDisplayWidth(display.id, screen) as c_uint;
			let height = xlib::XDisplayHeight(display.id, screen) as c_uint;
			let black  = xlib::XBlackPixelOfScreen(xlib::XScreenOfDisplay(display.id, screen));

			// We need to pick the visual even if the context isn't actually created
			// by the locker so the right colormap can be defined, as well as the
			// window's visual.
			let info = glx::glXChooseVisual(display.id, screen,
				[glx::GLX_RGBA, glx::GLX_DEPTH_SIZE, 24, glx::GLX_DOUBLEBUFFER, 0].as_ptr() as *mut _)
					.as_mut().ok_or(error::Locker::Visual)?;

			let colormap = xlib::XCreateColormap(display.id, root, (*info).visual, xlib::AllocNone);

			let id = {
				let mut attrs = mem::zeroed(): xlib::XSetWindowAttributes;
				let mut mask  = 0;

				mask |= xlib::CWColormap;
				attrs.colormap = colormap;

				mask |= xlib::CWBackingStore;
				attrs.backing_store = xlib::NotUseful;

				mask |= xlib::CWBackingPixel;
				attrs.backing_pixel = black;

				mask |= xlib::CWBorderPixel;
				attrs.border_pixel = black;

				mask |= xlib::CWOverrideRedirect;
				attrs.override_redirect = 1;

				mask |= xlib::CWEventMask;
				attrs.event_mask = xlib::KeyPressMask | xlib::KeyReleaseMask |
					xlib::ButtonPressMask | xlib::ButtonReleaseMask |
					xlib::PointerMotionMask | xlib::ExposureMask;

				xlib::XCreateWindow(display.id, root, 0, 0, width, height, 0, (*info).depth,
					xlib::InputOutput as c_uint, (*info).visual, mask, &mut attrs)
			};

			// Make sure the window background is black, this does not clear the window.
			xlib::XSetWindowBackground(display.id, id, black);

			// Set window property to mark the window as ours.
			xlib::XChangeProperty(display.id, id, display.atoms.saver, xlib::XA_CARDINAL, 32, xlib::PropModeReplace,
				&xlib::True as *const _ as *const _, 1);

			// Make the cursor invisible.
			let cursor = {
				let bit    = xlib::XCreatePixmapFromBitmapData(display.id, id, b"\x00".as_ptr() as *const _ as *mut _, 1, 1, black, black, 1);
				let cursor = xlib::XCreatePixmapCursor(display.id, bit, bit, &mut mem::zeroed(), &mut mem::zeroed(), 0, 0);

				xlib::XFreePixmap(display.id, bit);
				xlib::XDefineCursor(display.id, id, cursor);

				cursor
			};

			let im = xlib::XOpenIM(display.id, ptr::null_mut(), ptr::null_mut(), ptr::null_mut())
				.as_mut().ok_or(error::Locker::IM)?;

			let ic = util::with("inputStyle", |input_style|
				util::with("clientWindow", |client_window|
					xlib::XCreateIC(im, input_style, xlib::XIMPreeditNothing | xlib::XIMStatusNothing,
						client_window, id, ptr::null_mut::<()>())))
							.as_mut().ok_or(error::Locker::IC)?;

			Ok(Window {
				id:     id,
				width:  width,
				height: height,

				display: display,
				screen:  screen,
				root:    root,
				cursor:  cursor,
				im:      im,
				ic:      ic,

				locked:   false,
				keyboard: false,
				pointer:  false,
			})
		}
	}

	/// Sanitize the window.
	pub fn sanitize(&mut self) {
		unsafe {
			if self.locked {
				// Try to grab the pointer again in case it wasn't grabbed when locking.
				if !self.pointer && self.grab(Grab::Pointer).is_ok() {
					self.pointer = true;
				}

				// Remap the window in case stuff like popups went above the locker.
				xlib::XMapRaised(self.display.id, self.id);
			}

			// TODO(meh): Actually sanitize.
			xlib::XSync(self.display.id, xlib::False);
		}
	}

	/// Resize the window.
	pub fn resize(&mut self, width: u32, height: u32) {
		unsafe {
			if self.width == width && self.height == height {
				return;
			}

			self.width  = width;
			self.height = height;

			xlib::XResizeWindow(self.display.id, self.id, width, height);
			xlib::XSync(self.display.id, xlib::False);
		}
	}

	/// Grab the given input.
	fn grab(&self, grab: Grab) -> error::Result<()> {
		unsafe {
			let result = match grab {
				Grab::Keyboard => {
					xlib::XGrabKeyboard(self.display.id, self.id, xlib::False,
						xlib::GrabModeAsync, xlib::GrabModeAsync, xlib::CurrentTime)
				}

				Grab::Pointer => {
					xlib::XGrabPointer(self.display.id, self.id, xlib::False,
						(xlib::ButtonPressMask | xlib::ButtonReleaseMask | xlib::PointerMotionMask) as c_uint,
						xlib::GrabModeAsync, xlib::GrabModeAsync, 0,
						xlib::XBlackPixelOfScreen(xlib::XDefaultScreenOfDisplay(self.display.id)),
						xlib::CurrentTime)
				}
			};

			match result {
				xlib::GrabSuccess =>
					Ok(()),

				xlib::AlreadyGrabbed =>
					Err(error::Grab::Conflict.into()),

				xlib::GrabNotViewable =>
					Err(error::Grab::Unmapped.into()),

				xlib::GrabFrozen =>
					Err(error::Grab::Frozen.into()),

				_ =>
					unreachable!()
			}
		}
	}

	/// Try to grab the given input with 1ms pauses.
	fn try_grab(&self, grab: Grab, tries: usize) -> error::Result<()> {
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

		unsafe {
			// Map the window and make sure it's raised.
			xlib::XMapRaised(self.display.id, self.id);
			xlib::XSync(self.display.id, xlib::False);

			// Try to grab the keyboard and mouse.
			self.keyboard = self.try_grab(Grab::Keyboard, 500).is_ok();
			self.pointer  = self.try_grab(Grab::Pointer, 500).is_ok();

			// Some retarded X11 applications grab the keyboard and pointer for long
			// periods of time for no reason, so try to change focus and grab again.
			if !self.keyboard || !self.pointer {
				warn!("could not grab keyboard or pointer, trying to change focus");

				xlib::XSetInputFocus(self.display.id, self.root, xlib::RevertToPointerRoot, xlib::CurrentTime);
				xlib::XSync(self.display.id, xlib::False);

				// Failing to grab the keyboard is fatal since the window manager or
				// other applications may be stealing our thunder.
				if !self.keyboard {
					if let Err(err) = self.try_grab(Grab::Keyboard, 500) {
						xlib::XUnmapWindow(self.display.id, self.id);

						error!("coult not grab keyboard: {:?}", err);
						return Err(err);
					}
					else {
						self.keyboard = true;
					}
				}

				// TODO(meh): Consider if failing to grab pointer should be fatal,
				//            probably not.
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
			xlib::XSelectInput(self.display.id, self.root, xlib::SubstructureNotifyMask);

			// If the display supports XRandr listen for screen change events.
			if self.display.randr.is_some() {
				xrandr::XRRSelectInput(self.display.id, self.id, xrandr::RRScreenChangeNotifyMask);
			}

			self.locked = true;

			Ok(())
		}
	}

	/// Make the window solid black.
	pub fn blank(&mut self) {
		unsafe {
			let gc    = xlib::XCreateGC(self.display.id, self.id, 0, ptr::null_mut());
			let black = xlib::XBlackPixelOfScreen(xlib::XScreenOfDisplay(self.display.id, self.screen));

			xlib::XSetForeground(self.display.id, gc, black);
			xlib::XFillRectangle(self.display.id, self.id, gc, 0, 0, self.width, self.height);
		}
	}

	/// Unlock the window, hiding and ungrabbing whatever.
	pub fn unlock(&mut self) -> error::Result<()> {
		if !self.locked {
			return Ok(());
		}

		unsafe {
			xlib::XUnmapWindow(self.display.id, self.id);
			self.locked = false;

			xlib::XUngrabKeyboard(self.display.id, xlib::CurrentTime);
			self.keyboard = false;

			xlib::XUngrabPointer(self.display.id, xlib::CurrentTime);
			self.pointer = false;

			Ok(())
		}
	}
}

impl Drop for Window {
	fn drop(&mut self) {
		unsafe {
			xlib::XDestroyWindow(self.display.id, self.id);
		}
	}
}
