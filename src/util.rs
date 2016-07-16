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

use std::time::Duration;
use std::ffi::CString;

use libc::c_char;

pub trait DurationExt {
	fn as_msecs(&self) -> u64;
	fn as_nanosecs(&self) -> u64;
}

impl DurationExt for Duration {
	fn as_msecs(&self) -> u64 {
		self.as_secs() * 1_000 + (self.subsec_nanos() / 1_000_000) as u64
	}

	fn as_nanosecs(&self) -> u64 {
		self.as_secs() * 1_000_000_000 + self.subsec_nanos() as u64
	}
}

pub fn with<S: AsRef<str>, T, F: FnOnce(*const c_char) -> T>(string: S, func: F) -> T {
	let string = CString::new(string.as_ref().as_bytes()).unwrap();
	func(string.as_ptr())
}
