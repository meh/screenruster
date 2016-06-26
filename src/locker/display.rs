use std::ptr;
use std::ffi::CStr;
use std::sync::Arc;

use libc::c_int;
use x11::{xlib, xrandr, dpms, xmd};

use error;
use config;

#[derive(Debug)]
pub struct Display {
	pub id:    *mut xlib::Display,
	pub randr: Option<Extension>,
	pub dpms:  Option<Extension>,
}

#[derive(Debug)]
pub struct Extension {
	pub event: c_int,
	pub error: c_int,
}

impl Display {
	/// Open the default display.
	pub fn open(config: config::Locker) -> error::Result<Arc<Display>> {
		unsafe {
			let id = xlib::XOpenDisplay(ptr::null()).as_mut().ok_or(error::Locker::Display)?;

			Ok(Arc::new(Display {
				id: id,

				randr: {
					let mut event = 0;
					let mut error = 0;

					if xrandr::XRRQueryExtension(id, &mut event, &mut error) == xlib::True {
						Some(Extension { event: event, error: error })
					}
					else {
						None
					}
				},

				dpms: {
					let mut event = 0;
					let mut error = 0;

					if dpms::DPMSQueryExtension(id, &mut event, &mut error) == xlib::True &&
					   dpms::DPMSCapable(id) == xlib::True
					{
						// DPMS needs to be enabled for `DPMSForceLevel` to actually work,
						// so we just put maximum timeout and handle the states ourselves.
						dpms::DPMSSetTimeouts(id, 0xffff, 0xffff, 0xffff);
						dpms::DPMSEnable(id);

						Some(Extension { event: event, error: error })
					}
					else {
						None
					}
				},
			}))
		}
	}

	/// Get the display name.
	pub fn name(&self) -> &str {
		unsafe {
			CStr::from_ptr(xlib::XDisplayString(self.id)).to_str().unwrap()
		}
	}

	/// Check if the monitor is powered on or not.
	pub fn is_powered(&self) -> bool {
		if self.dpms.is_some() {
			unsafe {
				let mut level = 0;
				let mut state = 0;

				dpms::DPMSInfo(self.id, &mut level, &mut state);

				if state == xlib::False as xmd::BOOL {
					return true;
				}

				match level {
					dpms::DPMSModeOn =>
						true,

					dpms::DPMSModeOff | dpms::DPMSModeStandby | dpms::DPMSModeSuspend =>
						false,

					_ =>
						unreachable!()
				}
			}
		}
		else {
			true
		}
	}

	/// Turn the monitor on or off.
	pub fn power(&self, value: bool) {
		if self.dpms.is_none() {
			return;
		}

		unsafe {
			dpms::DPMSForceLevel(self.id, if value { dpms::DPMSModeOn } else { dpms::DPMSModeOff });
			xlib::XSync(self.id, xlib::False);
		}
	}
}

unsafe impl Send for Display { }
unsafe impl Sync for Display { }

impl Drop for Display {
	fn drop(&mut self) {
		unsafe {
			xlib::XCloseDisplay(self.id);
		}
	}
}
