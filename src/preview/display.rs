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
use std::sync::Arc;
use std::ffi::CStr;

use x11::xlib;

use error;

#[derive(Debug)]
pub struct Display {
	pub id: *mut xlib::Display,
}

unsafe impl Send for Display { }
unsafe impl Sync for Display { }

impl Display {
	pub fn open() -> error::Result<Arc<Display>> {
		unsafe {
			let id = xlib::XOpenDisplay(ptr::null());

			Ok(Arc::new(Display {
				id: id
			}))
		}
	}

	pub fn name(&self) -> &str {
		unsafe {
			CStr::from_ptr(xlib::XDisplayString(self.id)).to_str().unwrap()
		}
	}
}

impl Drop for Display {
	fn drop(&mut self) {
		unsafe {
			xlib::XCloseDisplay(self.id);
		}
	}
}
