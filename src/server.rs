use std::thread;
use std::sync::mpsc::{Receiver, Sender, channel};

use dbus;

use error;
use config;

pub struct Server {
	receiver: Receiver<Event>,
	sender:   Sender<Event>,
}

#[derive(Debug)]
pub enum Event {
	Error(error::Error),
	Method(dbus::Message),
}

impl Server {
	pub fn spawn(config: config::Server) -> error::Result<Server> {
		let (sender, i_receiver) = channel();
		let (i_sender, receiver) = channel();

		thread::spawn(move || {
			let connection = dbus::Connection::get_private(dbus::BusType::Session).unwrap();
			connection.register_name("meh.screen.saver", 0).unwrap();

			for item in connection.iter(1_000_000) {
				match item {
					dbus::ConnectionItem::MethodCall(message) => {
						sender.send(Event::Method(message));
					}

					other => {
						info!("dbus: {:?}", other);
					}
				}
			}
		});

		Ok(Server {
			receiver: i_receiver,
			sender:   i_sender,
		})
	}
}

impl AsRef<Receiver<Event>> for Server {
	fn as_ref(&self) -> &Receiver<Event> {
		&self.receiver
	}
}

impl AsRef<Sender<Event>> for Server {
	fn as_ref(&self) -> &Sender<Event> {
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
