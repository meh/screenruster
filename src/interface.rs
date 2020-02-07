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

use std::path::Path;
use std::time::{SystemTime, Duration};
use std::thread;
use std::sync::Arc;
use std::ops::Deref;
use channel::{self, Receiver, Sender, SendError};

use dbus::{
	Message,
	blocking::{
		LocalConnection as Connection,
		stdintf::org_freedesktop_dbus::RequestNameReply,
		BlockingSender as _,
	},
	channel::{Sender as _}
};
use log::error;

use crate::error;
use crate::config;

/// The DBus interface.
///
/// It mimics the GNOME screensaver interface for simple integration with a
/// GNOME environment, and also implements some ScreenRuster specific
/// interfaces.
///
/// It listens for relevant system events:
///
/// - `PrepareForSleep` from SystemD
pub struct Interface {
	receiver: Receiver<Request>,
	sender:   Sender<Response>,
	signals:  Sender<Signal>,
}

#[derive(Debug)]
pub enum Request {
	/// Reload the configuration file.
	Reload(Option<String>),

	/// Lock the screen.
	Lock,

	/// Cycle the saver.
	Cycle,

	/// Simulate user activity.
	SimulateUserActivity,

	/// Inhibit the starting of screen saving.
	Inhibit {
		application: String,
		reason:      String,
	},

	/// Remove a previous Inhibit.
	UnInhibit(u32),

	/// Throttle the resource usage of the screen saving.
	Throttle {
		application: String,
		reason:      String,
	},

	/// Remove a previous Throttle.
	UnThrottle(u32),

	/// Suspend any screen saver activity.
	Suspend {
		application: String,
		reason:      String,
	},

	/// Remove a previous Suspend.
	Resume(u32),

	/// Change the active status of the screen saver.
	SetActive(bool),

	/// Get the active status of the screen saver.
	GetActive,

	/// Get how many seconds the screen saver has been active.
	GetActiveTime,

	/// Get the idle status of the session.
	GetSessionIdle,

	/// Get how many seconds the session has been idle.
	GetSessionIdleTime,

	/// The system is preparing for sleep or coming out of sleep.
	PrepareForSleep(Option<SystemTime>),
}

#[derive(Debug)]
pub enum Response {
	/// Whether the reload was successful or not.
	Reload(bool),

	/// The cookie for the inhibition.
	Inhibit(u32),

	/// The cookie for the throttle.
	Throttle(u32),

	/// The cookie for the suspend.
	Suspend(u32),

	/// Whether the screen is active or not.
	Active(bool),
	
	/// How many seconds the saver has been active.
	ActiveTime(u64),

	/// Whether the session is idle or not.
	SessionIdle(bool),

	/// How many seconds the session has been idle.
	SessionIdleTime(u64),
}

#[derive(Debug)]
pub enum Signal {
	/// The saver has been activated or deactivated.
	Active(bool),

	/// The session has become idle or active.
	SessionIdle(bool),

	/// An authentication request was initiated or completed.
	AuthenticationRequest(bool),
}

impl Interface {
	/// Send a reload request.
	pub fn reload<P: AsRef<Path>>(path: Option<P>) -> error::Result<()> {
		let mut message = Message::new_method_call(
				"meh.rust.ScreenSaver",
				"/meh/rust/ScreenSaver",
				"meh.rust.ScreenSaver",
				"Reload")?;

		if let Some(value) = path {
			message = message.append1(value.as_ref().to_string_lossy().into_owned());
		}

		Connection::new_session()?.send(message)?;

		Ok(())
	}

	/// Send a lock request.
	pub fn lock() -> error::Result<()> {
		Connection::new_session()?.send(Message::new_method_call(
			"org.gnome.ScreenSaver",
			"/org/gnome/ScreenSaver",
			"org.gnome.ScreenSaver",
			"Lock")?)?;

		Ok(())
	}

	/// Send an activation request.
	pub fn activate() -> error::Result<()> {
		Connection::new_session()?.send(Message::new_method_call(
			"org.gnome.ScreenSaver",
			"/org/gnome/ScreenSaver",
			"org.gnome.ScreenSaver",
			"SetActive")?
				.append1(true))?;

		Ok(())
	}

	/// Send a deactivation request.
	pub fn deactivate() -> error::Result<()> {
		Connection::new_session()?.send(Message::new_method_call(
			"org.gnome.ScreenSaver",
			"/org/gnome/ScreenSaver",
			"org.gnome.ScreenSaver",
			"SimulateUserActivity")?)?;

		Ok(())
	}

	/// Send an inhibition request.
	pub fn inhibit() -> error::Result<u32> {
		Connection::new_session()?.send_with_reply_and_block(Message::new_method_call(
			"org.gnome.ScreenSaver",
			"/org/gnome/ScreenSaver",
			"org.gnome.ScreenSaver",
			"Inhibit")?
				.append2("screenruster", "requested by user")
			, Duration::from_millis(5_000))?
		.get1::<u32>()
		.ok_or(dbus::Error::new_custom("inibhition", "wrong response").into())
	}

	/// Send an uninhibition request.
	pub fn uninhibit(cookie: u32) -> error::Result<()> {
		Connection::new_session()?.send(Message::new_method_call(
			"org.gnome.ScreenSaver",
			"/org/gnome/ScreenSaver",
			"org.gnome.ScreenSaver",
			"UnInhibit")?
				.append1(cookie))?;

		Ok(())
	}

	/// Send a throttle request.
	pub fn throttle() -> error::Result<u32> {
		Connection::new_session()?.send_with_reply_and_block(Message::new_method_call(
			"org.gnome.ScreenSaver",
			"/org/gnome/ScreenSaver",
			"org.gnome.ScreenSaver",
			"Throttle")?
				.append2("screenruster", "requested by user")
			, Duration::from_millis(5_000))?
		.get1::<u32>()
		.ok_or(dbus::Error::new_custom("throttle", "wrong response").into())
	}

	/// Send an unthrottle request.
	pub fn unthrottle(cookie: u32) -> error::Result<()> {
		Connection::new_session()?.send(Message::new_method_call(
			"org.gnome.ScreenSaver",
			"/org/gnome/ScreenSaver",
			"org.gnome.ScreenSaver",
			"UnThrottle")?
				.append1(cookie))?;

		Ok(())
	}

	/// Send a suspension request.
	pub fn suspend() -> error::Result<u32> {
		Connection::new_session()?.send_with_reply_and_block(Message::new_method_call(
			"meh.rust.ScreenSaver",
			"/meh/rust/ScreenSaver",
			"meh.rust.ScreenSaver",
			"Suspend")?
				.append2("screenruster", "requested by user")
			, Duration::from_millis(5_000))?
		.get1::<u32>()
		.ok_or(dbus::Error::new_custom("suspend", "wrong response").into())
	}

	/// Send a resume request.
	pub fn resume(cookie: u32) -> error::Result<()> {
		Connection::new_session()?.send(Message::new_method_call(
			"meh.rust.ScreenSaver",
			"/meh/rust/ScreenSaver",
			"meh.rust.ScreenSaver",
			"Resume")?
				.append1(cookie))?;

		Ok(())
	}

	/// Spawn a DBus interface with the given configuration.
	pub fn spawn(config: config::Interface) -> error::Result<Interface> {
		let (sender,   i_receiver) = channel::unbounded();
		let (i_sender, receiver)   = channel::unbounded();
		let (s_sender, signals)    = channel::unbounded();
		let (g_sender, g_receiver) = channel::unbounded::<error::Result<()>>();

		macro_rules! dbus {
			(connect system) => (
				Connection::new_system()
			);

			(connect session) => (
				match Connection::new_session() {
					Ok(value) => {
						value
					}

					Err(error) => {
						g_sender.send(Err(error.into())).unwrap();
						return;
					}
				}
			);

			(register $conn:expr, $name:expr) => (
				match $conn.request_name($name, false, false, true) {
					Ok(RequestNameReply::Exists) => {
						g_sender.send(Err(error::DBus::AlreadyRegistered.into())).unwrap();
						return;
					}

					Err(error) => {
						g_sender.send(Err(error.into())).unwrap();
						return;
					}

					Ok(value) => {
						value
					}
				}
			);

			(watch $conn:expr, $filter:expr) => (
				$conn.add_match_no_cb($filter)
			);

			(ready) => (
				g_sender.send(Ok(())).unwrap();
			);

			(check) => (
				g_receiver.recv().unwrap()
			);

			(try $body:expr) => (
				match $body {
					Ok(value) => {
						value
					}

					Err(err) => {
						error!("{:?}", err);
						return None;
					}
				}
			);
		}

		macro_rules! cloning {
			([$($var:ident),*] $closure:expr) => ({
				$(let $var = $var.clone();)*
				$closure
			});
		}

		// System DBus handler.
		{
			let sender = sender.clone();

			thread::spawn(move || {
				/// Inhibits system suspension temporarily.
				fn inhibit(c: &Connection) -> Option<dbus::arg::OwnedFd> {
					dbus!(try c.send_with_reply_and_block(dbus!(try Message::new_method_call(
						"org.freedesktop.login1",
						"/org/freedesktop/login1",
						"org.freedesktop.login1.Manager",
						"Inhibit"))
							.append1("sleep")
							.append1("ScreenRuster")
							.append1("Preparing for sleep.")
							.append1("delay"), Duration::from_millis(1_000)))
						.get1()
				}

				let system = dbus!(connect system).unwrap();

				// Delay the next suspension.
				let mut inhibitor = inhibit(&system);

				// Watch for PrepareForSleep events from SystemD.
				dbus!(watch system, "path='/org/freedesktop/login1',interface='org.freedesktop.login1.Manager',member='PrepareForSleep'").unwrap();

				#[derive(Debug)]
				pub struct PrepareForSleep {
					pub arg0: bool,
				}

				impl dbus::arg::AppendAll for PrepareForSleep {
					fn append(&self, i: &mut dbus::arg::IterAppend) {
						dbus::arg::RefArg::append(&self.arg0, i);
					}
				}

				impl dbus::arg::ReadAll for PrepareForSleep {
					fn read(i: &mut dbus::arg::Iter) -> Result<Self, dbus::arg::TypeMismatchError> {
						Ok(PrepareForSleep {
							arg0: i.read()?,
						})
					}
				}

				impl dbus::message::SignalArgs for PrepareForSleep {
					const NAME: &'static str = "PrepareForSleep";
					const INTERFACE: &'static str = "org.freedesktop.login1.Manager";
				}

				system.with_proxy("org.freedesktop.login1.Manager", "/org/freedesktop/login1", Duration::from_micros(5_000))
					.match_signal(|p: PrepareForSleep, _: &Connection, _: &Message| {
						sender.send(Request::PrepareForSleep(
							if p.arg0 { Some(SystemTime::now()) } else { None })).unwrap();

						// In case the system is suspending, unlock the suspension,
						// otherwise delay the next.
						if p.arg0 {
							inhibitor.take();
						}
						else {
							inhibitor = inhibit(&system);
						}

						true
					});
			});
		}

		// Session DBus handler.
		{
			let sender = sender.clone();

			thread::spawn(move || {
				let mut session = dbus!(connect session);
				let f           = dbus::tree::Factory::new_sync::<()>();

				dbus!(register session, "org.gnome.ScreenSaver");
				dbus!(register session, "meh.rust.ScreenSaver");
				dbus!(ready);

				// GNOME screensaver signals.
				let active = Arc::new(f.signal("ActiveChanged", ()).sarg::<bool, _>("status"));
				let idle   = Arc::new(f.signal("SessionIdleChanged", ()).sarg::<bool, _>("status"));
				let begin  = Arc::new(f.signal("AuthenticationRequestBegin", ()));
				let end    = Arc::new(f.signal("AuthenticationRequestEnd", ()));

				let tree = f.tree(())
					// ScreenRuster interface.
					.add(f.object_path("/meh/rust/ScreenSaver", ()).introspectable().add(f.interface("meh.rust.ScreenSaver", ())
						.add_m(f.method("Reload", (), cloning!([config, sender, receiver] move |m| {
							if config.ignores("reload") {
								return Err(dbus::tree::MethodErr::failed(&"Reload is ignored"));
							}

							sender.send(Request::Reload(m.msg.get1())).unwrap();

							if let Response::Reload(value) = receiver.recv().unwrap() {
								Ok(vec![m.msg.method_return().append1(value)])
							}
							else {
								unreachable!();
							}
						})).inarg::<String, _>("path").outarg::<bool, _>("success"))

						.add_m(f.method("Suspend", (), cloning!([config, sender, receiver] move |m| {
							if config.ignores("suspend") {
								return Err(dbus::tree::MethodErr::failed(&"Suspend is ignored"));
							}

							if let (Some(application), Some(reason)) = m.msg.get2() {
								sender.send(Request::Suspend {
									application: application,
									reason:      reason
								}).unwrap();

								if let Response::Suspend(value) = receiver.recv().unwrap() {
									Ok(vec![m.msg.method_return().append1(value)])
								}
								else {
									unreachable!();
								}
							}
							else {
								Err(dbus::tree::MethodErr::no_arg())
							}
						})).in_args(vec![dbus::Signature::make::<String>(), dbus::Signature::make::<String>()]))

						.add_m(f.method("Resume", (), cloning!([config, sender] move |m| {
							if config.ignores("suspend") {
								return Err(dbus::tree::MethodErr::failed(&"Suspend is ignored"));
							}

							if let Some(cookie) = m.msg.get1() {
								sender.send(Request::Resume(cookie)).unwrap();

								Ok(vec![m.msg.method_return()])
							}
							else {
								Err(dbus::tree::MethodErr::no_arg())
							}
						})).inarg::<u32, _>("cookie"))))

					// GNOME screensaver interface.
					.add(f.object_path("/org/gnome/ScreenSaver", ()).introspectable().add(f.interface("org.gnome.ScreenSaver", ())
						.add_m(f.method("Lock", (), cloning!([sender] move |m| {
							sender.send(Request::Lock).unwrap();

							Ok(vec![m.msg.method_return()])
						})))

						.add_m(f.method("Cycle", (), cloning!([sender] move |m| {
							sender.send(Request::Cycle).unwrap();

							Ok(vec![m.msg.method_return()])
						})))

						.add_m(f.method("SimulateUserActivity", (), cloning!([sender] move |m| {
							sender.send(Request::SimulateUserActivity).unwrap();

							Ok(vec![m.msg.method_return()])
						})))

						.add_m(f.method("Inhibit", (), cloning!([config, sender, receiver] move |m| {
							if config.ignores("inhibit") {
								return Err(dbus::tree::MethodErr::failed(&"Inhibit is ignored"));
							}

							if let (Some(application), Some(reason)) = m.msg.get2() {
								sender.send(Request::Inhibit {
									application: application,
									reason:      reason
								}).unwrap();

								if let Response::Inhibit(value) = receiver.recv().unwrap() {
									Ok(vec![m.msg.method_return().append1(value)])
								}
								else {
									unreachable!();
								}
							}
							else {
								Err(dbus::tree::MethodErr::no_arg())
							}
						})).in_args(vec![dbus::Signature::make::<String>(), dbus::Signature::make::<String>()]))

						.add_m(f.method("UnInhibit", (), cloning!([config, sender] move |m| {
							if config.ignores("inhibit") {
								return Err(dbus::tree::MethodErr::failed(&"Inhibit is ignored"));
							}

							if let Some(cookie) = m.msg.get1() {
								sender.send(Request::UnInhibit(cookie)).unwrap();

								Ok(vec![m.msg.method_return()])
							}
							else {
								Err(dbus::tree::MethodErr::no_arg())
							}
						})).inarg::<u32, _>("cookie"))

						.add_m(f.method("Throttle", (), cloning!([config, sender, receiver] move |m| {
							if config.ignores("throttle") {
								return Err(dbus::tree::MethodErr::failed(&"Inhibit is ignored"));
							}

							if let (Some(application), Some(reason)) = m.msg.get2() {
								sender.send(Request::Throttle {
									application: application,
									reason:      reason
								}).unwrap();

								if let Response::Throttle(value) = receiver.recv().unwrap() {
									Ok(vec![m.msg.method_return().append1(value)])
								}
								else {
									unreachable!();
								}
							}
							else {
								Err(dbus::tree::MethodErr::no_arg())
							}
						})).in_args(vec![dbus::Signature::make::<String>(), dbus::Signature::make::<String>()]))

						.add_m(f.method("UnThrottle", (), cloning!([config, sender] move |m| {
							if config.ignores("throttle") {
								return Err(dbus::tree::MethodErr::failed(&"Inhibit is ignored"));
							}

							if let Some(cookie) = m.msg.get1() {
								sender.send(Request::UnThrottle(cookie)).unwrap();

								Ok(vec![m.msg.method_return()])
							}
							else {
								Err(dbus::tree::MethodErr::no_arg())
							}
						})).inarg::<u32, _>("cookie"))

						.add_m(f.method("SetActive", (), cloning!([sender] move |m| {
							if let Some(value) = m.msg.get1() {
								sender.send(Request::SetActive(value)).unwrap();

								Ok(vec![m.msg.method_return()])
							}
							else {
								Err(dbus::tree::MethodErr::no_arg())
							}
						})).inarg::<bool, _>("active"))

						.add_m(f.method("GetActive", (), cloning!([sender, receiver] move |m| {
							sender.send(Request::GetActive).unwrap();

							if let Response::Active(value) = receiver.recv().unwrap() {
								Ok(vec![m.msg.method_return().append1(value)])
							}
							else {
								unreachable!();
							}
						})).outarg::<bool, _>("active"))

						.add_m(f.method("GetActiveTime", (), cloning!([sender, receiver] move |m| {
							sender.send(Request::GetActiveTime).unwrap();

							if let Response::ActiveTime(time) = receiver.recv().unwrap() {
								Ok(vec![m.msg.method_return().append1(time)])
							}
							else {
								unreachable!();
							}
						})).outarg::<u64, _>("time"))

						.add_m(f.method("GetSessionIdle", (), cloning!([sender, receiver] move |m| {
							sender.send(Request::GetSessionIdle).unwrap();

							if let Response::SessionIdle(value) = receiver.recv().unwrap() {
								Ok(vec![m.msg.method_return().append1(value)])
							}
							else {
								unreachable!();
							}
						})).outarg::<bool, _>("idle"))

						.add_m(f.method("GetSessionIdleTime", (), cloning!([sender, receiver] move |m| {
							sender.send(Request::GetSessionIdleTime).unwrap();

							if let Response::SessionIdleTime(time) = receiver.recv().unwrap() {
								Ok(vec![m.msg.method_return().append1(time)])
							}
							else {
								unreachable!();
							}
						})).outarg::<u64, _>("time"))

						.add_s(active.clone())
						.add_s(idle.clone())
						.add_s(begin.clone())
						.add_s(end.clone())));

				tree.start_receive(&session);

				loop {
					session.process(Duration::from_millis(500));

					while let Ok(signal) = signals.try_recv() {
						session.send(match signal {
							Signal::Active(status) =>
								active.msg(&"/meh/rust/ScreenSaver".into(), &"org.gnome.ScreenSaver".into()).append1(status),

							Signal::SessionIdle(status) =>
								idle.msg(&"/meh/rust/ScreenSaver".into(), &"org.gnome.ScreenSaver".into()).append1(status),

							Signal::AuthenticationRequest(true) =>
								begin.msg(&"/meh/rust/ScreenSaver".into(), &"org.gnome.ScreenSaver".into()),

							Signal::AuthenticationRequest(false) =>
								end.msg(&"/meh/rust/ScreenSaver".into(), &"org.gnome.ScreenSaver".into()),
						}).unwrap();
					}
				}
			});
		}

		dbus!(check)?;

		Ok(Interface {
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

impl Deref for Interface {
	type Target = Receiver<Request>;

	fn deref(&self) -> &Receiver<Request> {
		&self.receiver
	}
}
