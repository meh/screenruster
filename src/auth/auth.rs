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
use std::sync::mpsc::{Receiver, Sender, SendError, channel};

use users;

use error;
use config;
use super::Authenticate;

pub struct Auth {
	receiver: Receiver<Response>,
	sender:   Sender<Request>,
}

#[derive(Clone, Debug)]
pub enum Request {
	Authenticate(String),
}

#[derive(Clone, Debug)]
pub enum Response {
	Success,
	Failure,
}

impl Auth {
	pub fn spawn(config: config::Auth) -> error::Result<Auth> {
		let     user    = users::get_current_username().ok_or(error::Auth::UnknownUser)?;
		let mut methods = Vec::new(): Vec<Box<Authenticate>>;

		#[cfg(feature = "auth-internal")]
		methods.push(box super::internal::new(config.get("internal"))?);

		#[cfg(feature = "auth-pam")]
		methods.push(box super::pam::new(config.get("pam"))?);

		let (sender, i_receiver) = channel();
		let (i_sender, receiver) = channel();

		thread::spawn(move || {
			'main: while let Ok(request) = receiver.recv() {
				match request {
					Request::Authenticate(password) => {
						for method in &mut methods {
							if let Ok(true) = method.authenticate(&user, &password) {
								sender.send(Response::Success).unwrap();
								continue 'main;
							}
						}

						sender.send(Response::Failure).unwrap();
					}
				}
			}
		});

		Ok(Auth {
			receiver: i_receiver,
			sender:   i_sender,
		})
	}

	pub fn authenticate<S: AsRef<str>>(&self, password: S) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Authenticate(password.as_ref().to_string()))
	}
}

impl AsRef<Receiver<Response>> for Auth {
	fn as_ref(&self) -> &Receiver<Response> {
		&self.receiver
	}
}

impl AsRef<Sender<Request>> for Auth {
	fn as_ref(&self) -> &Sender<Request> {
		&self.sender
	}
}
