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

use std::io::{self, BufRead, BufReader, Write};
use std::process::{self, Child, Command, Stdio};
use std::ops::Deref;
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, TryRecvError, Sender, SendError, channel};

use toml;
use log;
use api::{self, json};
pub use api::{Password, Pointer};

macro_rules! json {
	($body:expr) => (
		if let Some(value) = $body {
			value
		}
		else {
			continue;
		}
	);
}

use error;

pub struct Saver {
	process:  Arc<Mutex<Child>>,
	receiver: Option<Receiver<Response>>,
	sender:   Sender<Request>,

	started: bool,
	stopped: bool,
}

#[derive(Debug)]
pub enum Request {
	Forward(api::Request),
	Exit,
}

#[derive(Debug)]
pub enum Response {
	Forward(api::Response),
	Exit(Exit),
}

#[derive(Debug)]
pub struct Exit {
	status: process::ExitStatus,
	sender: Sender<Response>,
}

impl Deref for Exit {
	type Target = process::ExitStatus;

	fn deref(&self) -> &Self::Target {
		&self.status
	}
}

impl Saver {
	/// Spawn the saver with the given name.
	pub fn spawn<S: AsRef<str>>(name: S) -> error::Result<Saver> {
		let child = Arc::new(Mutex::new(Command::new(format!("screenruster-saver-{}", name.as_ref()))
			.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
			.spawn()?));

		let (sender, i_receiver) = channel();
		let (i_sender, receiver) = channel();

		// Reader.
		{
			let input    = child.lock().unwrap().stdout.take().unwrap();
			let child    = child.clone();
			let internal = i_sender.clone();

			thread::spawn(move || {
				for line in BufReader::new(input).lines() {
					if line.is_err() {
						break;
					}

					if let Ok(message) = json::parse(&line.unwrap()) {
						sender.send(Response::Forward(match json!(message["type"].as_str()) {
							"initialized" => {
								api::Response::Initialized
							}

							"started" => {
								api::Response::Started
							}

							"stopped" => {
								api::Response::Stopped
							}

							_ =>
								continue
						})).unwrap();
					}
				}

				internal.send(Request::Exit).unwrap();

				let mut child = child.lock().unwrap();
				sender.send(Response::Exit(Exit {
					status: child.wait().unwrap(),
					sender: sender.clone()
				})).unwrap()
			});
		}

		// Writer.
		{
			let mut output = child.lock().unwrap().stdin.take().unwrap();

			thread::spawn(move || {
				while let Ok(request) = receiver.recv() {
					match request {
						Request::Forward(request) => {
							output.write_all(json::stringify(match request {
								api::Request::Config(config) => object!{
									"type"   => "config",
									"config" => config
								},

								api::Request::Target { display, screen, window } => object!{
									"type"    => "target",
									"display" => display,
									"screen"  => screen,
									"window"  => window
								},

								api::Request::Throttle(value) => object!{
									"type"     => "throttle",
									"throttle" => value
								},

								api::Request::Blank(value) => object!{
									"type"  => "blank",
									"blank" => value
								},

								api::Request::Resize { width, height } => object!{
									"type"   => "resize",
									"width"  => width,
									"height" => height
								},

								api::Request::Pointer(Pointer::Move { x, y }) => object!{
									"type" => "pointer",
									"move" => object!{
										"x" => x,
										"y" => y
									}
								},

								api::Request::Pointer(Pointer::Button { x, y, button, press }) => object!{
									"type"   => "pointer",
									"button" => object!{
										"x"      => x,
										"y"      => y,
										"button" => button,
										"press"  => press
									}
								},

								api::Request::Password(password) => object!{
									"type"     => "password",
									"password" => match password {
										Password::Insert  => "insert",
										Password::Delete  => "delete",
										Password::Reset   => "reset",
										Password::Check   => "check",
										Password::Success => "success",
										Password::Failure => "failure",
									}
								},

								api::Request::Start => object!{
									"type" => "start"
								},

								api::Request::Lock => object!{
									"type" => "lock"
								},

								api::Request::Stop => object!{
									"type" => "stop"
								},
							}).as_bytes()).unwrap();

							output.write_all(b"\n").unwrap();
						}

						Request::Exit => {
							break;
						}
					}
				}
			});
		}

		// Logger.
		{
			let input = child.lock().unwrap().stderr.take().unwrap();

			thread::spawn(move || {
				for line in BufReader::new(input).lines() {
					if line.is_err() {
						break;
					}

					if log_enabled!(log::LogLevel::Debug) {
						writeln!(&mut io::stderr(), "{}", line.unwrap()).unwrap();
					}
				}
			});
		}

		Ok(Saver {
			process:  child,
			receiver: Some(i_receiver),
			sender:   i_sender,

			started: false,
			stopped: false,
		})
	}

	/// Check if the saver was requested to start.
	pub fn was_started(&self) -> bool {
		self.started
	}

	/// Check if the saver was requested to stop.
	pub fn was_stopped(&self) -> bool {
		self.stopped
	}

	/// Kill the saver process.
	pub fn kill(&mut self) {
		let _ = self.process.lock().unwrap().kill();
	}

	/// Take the internal receiver.
	pub fn take(&mut self) -> Option<Receiver<Response>> {
		self.receiver.take()
	}

	/// Try to receive a message from the saver.
	pub fn recv(&mut self) -> Option<Response> {
		if let Some(receiver) = self.receiver.as_ref() {
			match receiver.try_recv() {
				Ok(response) =>
					Some(response),

				Err(TryRecvError::Empty) =>
					None,

				Err(TryRecvError::Disconnected) => {
					None
				}
			}
		}
		else {
			None
		}
	}

	/// Send the API request.
	fn send(&mut self, request: api::Request) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Forward(request))
	}

	/// Configure the saver.
	pub fn config(&mut self, config: toml::Table) -> Result<(), SendError<Request>> {
		fn convert(value: &toml::Value) -> json::JsonValue {
			match *value {
				toml::Value::String(ref value) | toml::Value::Datetime(ref value) =>
					value.clone().into(),

				toml::Value::Integer(value) =>
					value.into(),

				toml::Value::Float(value) =>
					value.into(),

				toml::Value::Boolean(value) =>
					value.into(),

				toml::Value::Array(ref value) =>
					json::JsonValue::Array(value.iter().map(|v| convert(v)).collect()),

				toml::Value::Table(ref value) =>
					json::JsonValue::Object(value.iter().map(|(k, v)| (k.clone(), convert(v))).collect()),
			}
		}

		self.send(api::Request::Config(convert(&toml::Value::Table(config))))
	}

	/// Select the rendering target for the saver.
	pub fn target<S: AsRef<str>>(&mut self, display: S, screen: i32, window: u64) -> Result<(), SendError<Request>> {
		self.send(api::Request::Target {
			display: display.as_ref().into(),
			screen:  screen,
			window:  window,
		})
	}

	/// Throttle or unthrottle the saer.
	pub fn throttle(&mut self, value: bool) -> Result<(), SendError<Request>> {
		self.send(api::Request::Throttle(value))
	}

	/// Tell the saver the screen has been blanked or unblanked.
	pub fn blank(&mut self, value: bool) -> Result<(), SendError<Request>> {
		self.send(api::Request::Blank(value))
	}

	/// Resize the saver.
	pub fn resize(&mut self, width: u32, height: u32) -> Result<(), SendError<Request>> {
		self.send(api::Request::Resize {
			width:  width,
			height: height,
		})
	}

	/// Send a pointer event.
	pub fn pointer(&mut self, pointer: Pointer) -> Result<(), SendError<Request>> {
		self.send(api::Request::Pointer(pointer))
	}

	/// Send a password event.
	pub fn password(&mut self, password: Password) -> Result<(), SendError<Request>> {
		self.send(api::Request::Password(password))
	}

	/// Start the saver.
	pub fn start(&mut self) -> Result<(), SendError<Request>> {
		self.started = true;
		self.send(api::Request::Start)
	}

	/// Start the saver.
	pub fn lock(&mut self) -> Result<(), SendError<Request>> {
		self.send(api::Request::Lock)
	}

	/// Stop the saver.
	pub fn stop(&mut self) -> Result<(), SendError<Request>> {
		self.stopped = true;
		self.send(api::Request::Stop)
	}
}
