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
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, SendError, channel};

use dbus;

use error;
use config;

pub struct Server {
	receiver: Receiver<Request>,
	sender:   Sender<Response>,
	signals:  Sender<Signal>,
}

#[derive(Debug)]
pub enum Request {
	Lock,

	Cycle,

	SimulateUserActivity,

	Inhibit {
		application: String,
		reason:      String,
	},

	UnInhibit(u32),

	Throttle {
		application: String,
		reason:      String,
	},

	UnThrottle(u32),

	SetActive(bool),

	GetActive,
	GetActiveTime,

	GetSessionIdle,
	GetSessionIdleTime,
}

#[derive(Debug)]
pub enum Response {
	Inhibit(u32),
	Throttle(u32),

	Active(bool),
	ActiveTime(u64),

	SessionIdle(bool),
	SessionIdleTime(u64),
}

#[derive(Debug)]
pub enum Signal {
	Active(bool),
	SessionIdle(bool),
	AuthenticationRequest(bool),
}

impl Server {
	pub fn spawn(config: config::Server) -> error::Result<Server> {
		let (sender, i_receiver) = channel();
		let (i_sender, receiver) = channel();
		let (s_sender, signals)  = channel();

		thread::spawn(move || {
			let c = dbus::Connection::get_private(dbus::BusType::Session).unwrap();
			c.register_name("org.gnome.ScreenSaver", 0).unwrap();

			let f = dbus::tree::Factory::new_fn();

			let active = Arc::new(f.signal("ActiveChanged").sarg::<bool, _>("status"));
			let idle   = Arc::new(f.signal("SessionIdleChanged").sarg::<bool, _>("status"));
			let begin  = Arc::new(f.signal("AuthenticationRequestBegin"));
			let end    = Arc::new(f.signal("AuthenticationRequestEnd"));

			let tree = f.tree()
				.add(f.object_path("/org/gnome/ScreenSaver").introspectable().add(f.interface("org.gnome.ScreenSaver")
					.add_m(f.method("Lock", |_, _, _| {
						sender.send(Request::Lock).unwrap();

						Ok(vec![])
					}))

					.add_m(f.method("Cycle", |_, _, _| {
						sender.send(Request::Cycle).unwrap();

						Ok(vec![])
					}))

					.add_m(f.method("SimulateUserActivity", |_, _, _| {
						sender.send(Request::SimulateUserActivity).unwrap();

						Ok(vec![])
					}))

					.add_m(f.method("Inhibit", |m, _, _| {
						if config.ignore.contains("inhibit") {
							return Err(dbus::tree::MethodErr::failed(&"Inhibit is ignored"));
						}

						if let (Some(application), Some(reason)) = m.get2() {
							sender.send(Request::Inhibit {
								application: application,
								reason:      reason
							}).unwrap();

							if let Response::Inhibit(value) = receiver.recv().unwrap() {
								Ok(vec![m.method_return().append1(value)])
							}
							else {
								unreachable!();
							}
						}
						else {
							Err(dbus::tree::MethodErr::no_arg())
						}
					}).in_args(vec![dbus::Signature::make::<String>(), dbus::Signature::make::<String>()]))

					.add_m(f.method("UnInhibit", |m, _, _| {
						if config.ignore.contains("inhibit") {
							return Err(dbus::tree::MethodErr::failed(&"Inhibit is ignored"));
						}

						if let Some(cookie) = m.get1() {
							sender.send(Request::UnInhibit(cookie)).unwrap();

							Ok(vec![])
						}
						else {
							Err(dbus::tree::MethodErr::no_arg())
						}
					}).inarg::<u32, _>("cookie"))

					.add_m(f.method("Throttle", |m, _, _| {
						if config.ignore.contains("throttle") {
							return Err(dbus::tree::MethodErr::failed(&"Inhibit is ignored"));
						}

						if let (Some(application), Some(reason)) = m.get2() {
							sender.send(Request::Throttle {
								application: application,
								reason:      reason
							}).unwrap();

							if let Response::Throttle(value) = receiver.recv().unwrap() {
								Ok(vec![m.method_return().append1(value)])
							}
							else {
								unreachable!();
							}
						}
						else {
							Err(dbus::tree::MethodErr::no_arg())
						}
					}).in_args(vec![dbus::Signature::make::<String>(), dbus::Signature::make::<String>()]))

					.add_m(f.method("UnThrottle", |m, _, _| {
						if config.ignore.contains("throttle") {
							return Err(dbus::tree::MethodErr::failed(&"Inhibit is ignored"));
						}

						if let Some(cookie) = m.get1() {
							sender.send(Request::UnThrottle(cookie)).unwrap();

							Ok(vec![])
						}
						else {
							Err(dbus::tree::MethodErr::no_arg())
						}
					}).inarg::<u32, _>("cookie"))

					.add_m(f.method("SetActive", |m, _, _| {
						if let Some(value) = m.get1() {
							sender.send(Request::SetActive(value)).unwrap();

							Ok(vec![])
						}
						else {
							Err(dbus::tree::MethodErr::no_arg())
						}
					}).inarg::<bool, _>("active"))

					.add_m(f.method("GetActive", |m, _, _| {
						sender.send(Request::GetActive).unwrap();

						if let Response::Active(value) = receiver.recv().unwrap() {
							Ok(vec![m.method_return().append1(value)])
						}
						else {
							unreachable!();
						}
					}).outarg::<bool, _>("active"))

					.add_m(f.method("GetActiveTime", |m, _, _| {
						sender.send(Request::GetActiveTime).unwrap();

						if let Response::ActiveTime(time) = receiver.recv().unwrap() {
							Ok(vec![m.method_return().append1(time)])
						}
						else {
							unreachable!();
						}
					}).outarg::<u64, _>("time"))

					.add_m(f.method("GetSessionIdle", |m, _, _| {
						sender.send(Request::GetSessionIdle).unwrap();

						if let Response::SessionIdle(value) = receiver.recv().unwrap() {
							Ok(vec![m.method_return().append1(value)])
						}
						else {
							unreachable!();
						}
					}).outarg::<bool, _>("idle"))

					.add_m(f.method("GetSessionIdleTime", |m, _, _| {
						sender.send(Request::GetSessionIdleTime).unwrap();

						if let Response::SessionIdleTime(time) = receiver.recv().unwrap() {
							Ok(vec![m.method_return().append1(time)])
						}
						else {
							unreachable!();
						}
					}).outarg::<u64, _>("time"))

					.add_s_arc(active.clone())
					.add_s_arc(idle.clone())
					.add_s_arc(begin.clone())
					.add_s_arc(end.clone())));

			tree.set_registered(&c, true).unwrap();

			for item in tree.run(&c, c.iter(100)) {
				if let dbus::ConnectionItem::Nothing = item {
					while let Ok(signal) = signals.try_recv() {
						c.send(match signal {
							Signal::Active(status) => {
								active.msg().append1(status)
							}

							Signal::SessionIdle(status) => {
								idle.msg().append1(status)
							}

							Signal::AuthenticationRequest(true) => {
								begin.msg()
							}

							Signal::AuthenticationRequest(false) => {
								end.msg()
							}
						}).unwrap();
					}
				}
			}
		});

		Ok(Server {
			receiver: i_receiver,
			sender:   i_sender,
			signals:  s_sender,
		})
	}

	pub fn response(&self, value: Response) -> Result<(), SendError<Response>> {
		self.sender.send(value)
	}

	pub fn signal(&self, value: Signal) -> Result<(), SendError<Signal>> {
		self.signals.send(value)
	}
}

impl AsRef<Receiver<Request>> for Server {
	fn as_ref(&self) -> &Receiver<Request> {
		&self.receiver
	}
}

impl AsRef<Sender<Response>> for Server {
	fn as_ref(&self) -> &Sender<Response> {
		&self.sender
	}
}

impl AsRef<Sender<Signal>> for Server {
	fn as_ref(&self) -> &Sender<Signal> {
		&self.signals
	}
}
