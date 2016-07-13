use std::mem;
use std::ptr;
use std::ffi::{CStr, CString};

use toml;
use pam;
use libc::{c_char, c_int, c_void, size_t};
use libc::{calloc, free, strdup};

use error;
use super::Authenticate;

macro_rules! pam {
	($handle:ident) => (
		pam::end($handle, pam::PamReturnCode::SUCCESS);
	);

	($handle:ident, $body:expr) => (
		match $body {
			pam::PamReturnCode::SUCCESS =>
				Ok(()),

			error => {
				pam::end($handle, error);
				Err(error::auth::Pam(error))
			}
		}
	);

	($body:expr) => (
		match $body {
			pam::PamReturnCode::SUCCESS =>
				Ok(()),

			error =>
				Err(error::auth::Pam(error))
		}
	);
}

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

			pam!(pam::start(b"screenruster\x00".as_ptr() as *const _, ptr::null(), &conv, &mut handle))?;
			pam!(handle, pam::set_item(handle, pam::PamItemType::TTY, strdup(b":0.0\x00".as_ptr() as *const _) as *mut _))?;
			pam!(handle, pam::authenticate(handle, pam::PamFlag::NONE))?;

			// On some systems account management is not configured properly, but
			// some PAM modules require it to be called to work properly, so make the
			// erroring optional.
			if self.accounts {
				pam!(handle, pam::acct_mgmt(handle, pam::PamFlag::NONE))?;
				pam!(handle, pam::setcred(handle, pam::PamFlag::REINITIALIZE_CRED))?;
				pam!(handle);
			}
			else {
				if pam!(handle, pam::acct_mgmt(handle, pam::PamFlag::NONE)).is_ok() {
					pam!(handle);
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
