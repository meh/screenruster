use std::thread;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::ops::Deref;

use dbus::{Connection, BusType};
use dbus::tree::{Tree, Factory, MethodFn};

use error;
use config;

pub struct Server {
	receiver: Receiver<Event>,
	sender:   Sender<Event>,
}

#[derive(Debug)]
pub enum Event {
	Error(error::Error),
}

impl Server {
	pub fn spawn(config: config::Server) -> error::Result<Server> {
		let (sender, i_receiver) = channel();
		let (i_sender, receiver) = channel();

		thread::spawn(move || {
			let connection = Connection::get_private(BusType::Session).unwrap();
			connection.register_name("meh.screen.saver", 0).unwrap();

			for event in connection.iter(1_000) {
				println!("{:?}", event);
			}
		});

		Ok(Server {
			receiver: i_receiver,
			sender:   i_sender,
		})
	}
}

impl Deref for Server {
	type Target = Receiver<Event>;

	fn deref(&self) -> &Self::Target {
		&self.receiver
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
