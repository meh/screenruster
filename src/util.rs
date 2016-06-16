use std::time::Duration;
use std::ffi::CString;

use libc::c_char;

pub trait DurationExt {
	fn as_msecs(&self) -> u64;
	fn as_nanosecs(&self) -> u64;
}

impl DurationExt for Duration {
	fn as_msecs(&self) -> u64 {
		self.as_secs() * 1_000 + (self.subsec_nanos() / 1_000) as u64
	}

	fn as_nanosecs(&self) -> u64 {
		self.as_secs() * 1_000_000 + self.subsec_nanos() as u64
	}
}

pub fn with<S: AsRef<str>, T, F: FnOnce(*const c_char) -> T>(string: S, func: F) -> T {
	let string = CString::new(string.as_ref().as_bytes()).unwrap();
	func(string.as_ptr())
}
