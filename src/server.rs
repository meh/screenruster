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
use std::sync::mpsc::{Receiver, Sender, channel};

#[cfg(feature = "dbus")]
use dbus;

use error;
use config;

pub struct Server {
	receiver: Receiver<Response>,
	sender:   Sender<Request>,
}

#[derive(Debug)]
pub enum Request {

}

#[derive(Debug)]
pub enum Response {
	Error(error::Error),
}

impl Server {
	pub fn spawn(config: config::Server) -> error::Result<Server> {
		let (sender, i_receiver) = channel();
		let (i_sender, receiver) = channel();

		#[cfg(feature = "dbus")]
		thread::spawn(move || {
			let connection = dbus::Connection::get_private(dbus::BusType::Session).unwrap();
			connection.register_name("meh.screen.saver", 0).unwrap();

			for item in connection.iter(1_000_000) {
				match item {
					dbus::ConnectionItem::MethodCall(message) => {
						sender.send(Response::Method(message)).unwrap();
					}

					other => {
						info!("dbus: {:?}", other);
					}
				}
			}
		});

		#[cfg(not(feature = "dbus"))]
		thread::spawn(move || {
			let _ = sender;
			let _ = receiver;

			loop {
				thread::sleep(::std::time::Duration::from_secs(3600));
			}
		});

		Ok(Server {
			receiver: i_receiver,
			sender:   i_sender,
		})
	}
}

impl AsRef<Receiver<Response>> for Server {
	fn as_ref(&self) -> &Receiver<Response> {
		&self.receiver
	}
}

impl AsRef<Sender<Request>> for Server {
	fn as_ref(&self) -> &Sender<Request> {
		&self.sender
	}
}

// This goes in an helper function.
//
//		let f    = Factory::new_fn();
//		let tree = f.tree()
//			.add(f.object_path("/start").introspectable().add(
//				f.interface("meh.screen.saver").add_m(
//					f.method("Start", |m, _, _| Ok(Vec::new())))));
//
//
