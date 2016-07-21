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

use std::mem;
use std::sync::Arc;

use libc::{c_int, c_uint};
use x11::{xlib, glx};

use error;
use super::Display;

#[derive(Debug)]
pub struct Window {
	pub id:     xlib::Window,
	pub width:  u32,
	pub height: u32,

	pub display: Arc<Display>,
	pub screen:  c_int,
	pub root:    xlib::Window,
}

unsafe impl Send for Window { }
unsafe impl Sync for Window { }

impl Window {
	pub fn create(display: Arc<Display>, screen: c_int) -> error::Result<Window> {
		unsafe {
			let root   = xlib::XRootWindow(display.id, screen);
			let width  = (xlib::XDisplayWidth(display.id, screen) as f32 / 1.2) as u32;
			let height = (xlib::XDisplayHeight(display.id, screen) as f32 / 1.2) as u32;
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

				mask |= xlib::CWEventMask;
				attrs.event_mask = xlib::KeyPressMask | xlib::KeyReleaseMask |
					xlib::ButtonPressMask | xlib::ButtonReleaseMask |
					xlib::PointerMotionMask | xlib::ExposureMask;

				xlib::XCreateWindow(display.id, root, 0, 0, width, height, 0, (*info).depth,
					xlib::InputOutput as c_uint, (*info).visual, mask, &mut attrs)
			};

			// Set normal hints.
			{
				let mut hints = xlib::XAllocSizeHints();

				(*hints).min_aspect.x = width as c_int;
				(*hints).min_aspect.y = height as c_int;

				(*hints).max_aspect.x = width as c_int;
				(*hints).max_aspect.y = height as c_int;

				(*hints).flags = xlib::PAspect;

				xlib::XSetWMNormalHints(display.id, id, hints);
			}

			// Make sure the window background is black, this does not clear the window.
			xlib::XSetWindowBackground(display.id, id, black);

			Ok(Window {
				id:     id,
				width:  width,
				height: height,

				display: display,
				screen:  screen,
				root:    root,
			})
		}
	}

	pub fn show(&self) {
		unsafe {
			xlib::XMapWindow(self.display.id, self.id);
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
