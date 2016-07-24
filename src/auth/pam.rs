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
use pam;
use libc::{c_char, c_int, c_void, size_t};
use libc::{calloc, free, strdup};

use error;
use super::Authenticate;

pub struct Auth {
	accounts: bool,
}

pub fn new(_config: toml::Table) -> error::Result<Auth> {
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
		let user     = CString::new(user).unwrap();
		let password = CString::new(password).unwrap();

		unsafe {
			let mut handle = mem::uninitialized();
			let     conv   = pam::PamConversation {
				conv:     Some(conversation),
				data_ptr: &Info { user: user.as_ptr(), password: password.as_ptr() } as *const _ as *mut _,
			};

			macro_rules! pam {
				(check $body:expr) => (
					match $body {
						pam::PamReturnCode::SUCCESS =>
							Ok(()),

						error =>
							Err(error::auth::Pam(error))
					}
				);

				(checked $body:expr) => (
					match $body {
						pam::PamReturnCode::SUCCESS =>
							Ok(()),

						error => {
							pam::end(handle, error);
							Err(error::auth::Pam(error))
						}
					}
				);

				(start) => (
					pam!(check pam::start(b"screenruster\x00".as_ptr() as *const _, ptr::null(), &conv, &mut handle))
				);

				(set_item $ty:ident => $value:expr) => (
					pam!(checked pam::set_item(handle, pam::PamItemType::$ty, strdup($value.as_ptr() as *const _) as *mut _))
				);

				(authenticate) => (
					pam!(checked pam::authenticate(handle, pam::PamFlag::NONE))
				);

				(end) => (
					pam::end(handle, pam::PamReturnCode::SUCCESS);
				);

				($name:ident $flag:ident) => (
					pam!(checked pam::$name(handle, pam::PamFlag::$flag))
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
				pam!(acct_mgmt)?;
				pam!(setcred REINITIALIZE_CRED)?;
				pam!(end);
			}
			else {
				if pam!(acct_mgmt).is_ok() {
					pam!(end);
				}
			}

			Ok(true)
		}
	}
}

extern "C" fn conversation(count: c_int, messages: *mut *mut pam::PamMessage, responses: *mut *mut pam::PamResponse, data: *mut c_void) -> c_int {
	unsafe {
		let     info   = &*(data as *mut Info as *const Info);
		let mut result = pam::PamReturnCode::SUCCESS;

		*responses = calloc(count as size_t, mem::size_of::<pam::PamResponse>() as size_t).as_mut().unwrap() as *mut _ as *mut _;

		for i in 0 .. count as isize {
			let message  = &**messages.offset(i);
			let response = &mut *((*responses).offset(i));

			match pam::PamMessageStyle::from(message.msg_style) {
				// Probably the username, since it wants us to echo it.
				pam::PamMessageStyle::PROMPT_ECHO_ON => {
					response.resp = strdup(info.user);
				}

				// Probably the password, since it doesn't want us to echo it.
				pam::PamMessageStyle::PROMPT_ECHO_OFF => {
					response.resp = strdup(info.password);
				}

				pam::PamMessageStyle::ERROR_MSG => {
					result = pam::PamReturnCode::CONV_ERR;
					error!("{}", String::from_utf8_lossy(CStr::from_ptr(message.msg).to_bytes()));
				}

				pam::PamMessageStyle::TEXT_INFO => {
					info!("{}", String::from_utf8_lossy(CStr::from_ptr(message.msg).to_bytes()));
				}
			}
		}

		if result != pam::PamReturnCode::SUCCESS {
			free(*responses as *mut _);
		}

		result as c_int
	}
}
