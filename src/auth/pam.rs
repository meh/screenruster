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
use std::ptr;
use std::ffi::{CStr, CString};

use toml;
use log::{info, error};
use pam::types::*;
use pam::raw::*;
use libc::{c_char, c_int, c_void, size_t};
use libc::{calloc, free, strdup};

use crate::error;
use super::Authenticate;

pub struct Auth {
	accounts: bool,
}

pub fn new(_config: toml::value::Table) -> error::Result<Auth> {
	Ok(Auth {
		accounts: cfg!(feature = "auth-pam-accounts"),
	})
}

struct Info {
	user:     *const c_char,
	password: *const c_char,
}

impl Authenticate for Auth {
	fn authenticate(&mut self, user: &str, password: &str) -> error::Result<bool> {
		let user     = CString::new(user)?;
		let password = CString::new(password)?;

		unsafe {
			let mut handle = mem::MaybeUninit::<*mut PamHandle>::uninit();
			let     conv   = PamConversation {
				conv:     Some(conversation),
				data_ptr: &Info { user: user.as_ptr(), password: password.as_ptr() } as *const _ as *mut _,
			};

			macro_rules! pam {
				(check $body:expr) => (
					match $body.into() {
						PamReturnCode::SUCCESS =>
							Ok(()),

						error =>
							Err(error::auth::Pam(error))
					}
				);

				(checked $body:expr) => (
					match $body.into() {
						PamReturnCode::SUCCESS =>
							Ok(()),

						error => {
							pam_end(&mut *handle.assume_init(), error as i32);
							Err(error::auth::Pam(error))
						}
					}
				);

				(start) => (
					pam!(check pam_start(b"screenruster\x00".as_ptr() as *const _, ptr::null(), &conv, handle.as_mut_ptr() as *mut *const _))
				);

				(set_item $ty:ident => $value:expr) => (
					pam!(checked pam_set_item(&mut *handle.assume_init(), PamItemType::$ty as i32, strdup($value.as_ptr() as *const _) as *mut _))
				);

				(authenticate) => (
					pam!(checked pam_authenticate(&mut *handle.assume_init(), PamFlag::NONE as i32))
				);

				(end) => (
					pam_end(&mut *handle.assume_init(), PamReturnCode::SUCCESS as i32);
				);

				($name:ident $flag:ident) => (
					pam!(checked $name(&mut *handle.assume_init(), PamFlag::$flag as i32))
				);

				($name:ident) => (
					pam!($name NONE)
				);
			}

			pam!(start)?;
			pam!(set_item TTY => b":0.0\0")?;
			pam!(authenticate)?;

			// On some systems account management is not configured properly, but
			// some PAM modules require it to be called to work properly, so make the
			// erroring optional.
			if self.accounts {
				pam!(pam_acct_mgmt)?;
				pam!(pam_setcred REINITIALIZE_CRED)?;
				pam!(end);
			}
			else {
				if pam!(pam_acct_mgmt).is_ok() {
					pam!(end);
				}
			}

			Ok(true)
		}
	}
}

extern "C" fn conversation(count: c_int, messages: *mut *mut PamMessage, responses: *mut *mut PamResponse, data: *mut c_void) -> c_int {
	unsafe {
		let     info   = &*(data as *mut Info as *const Info);
		let mut result = PamReturnCode::SUCCESS;

		*responses = calloc(count as size_t, mem::size_of::<PamResponse>() as size_t).as_mut().unwrap() as *mut _ as *mut _;

		for i in 0 .. count as isize {
			let message  = &**messages.offset(i);
			let response = &mut *((*responses).offset(i));

			match PamMessageStyle::from(message.msg_style) {
				// Probably the username, since it wants us to echo it.
				PamMessageStyle::PROMPT_ECHO_ON => {
					response.resp = strdup(info.user);
				}

				// Probably the password, since it doesn't want us to echo it.
				PamMessageStyle::PROMPT_ECHO_OFF => {
					response.resp = strdup(info.password);
				}

				PamMessageStyle::ERROR_MSG => {
					result = PamReturnCode::CONV_ERR;
					error!("{}", String::from_utf8_lossy(CStr::from_ptr(message.msg).to_bytes()));
				}

				PamMessageStyle::TEXT_INFO => {
					info!("{}", String::from_utf8_lossy(CStr::from_ptr(message.msg).to_bytes()));
				}
			}
		}

		if result != PamReturnCode::SUCCESS {
			free(*responses as *mut _);
		}

		result as c_int
	}
}
