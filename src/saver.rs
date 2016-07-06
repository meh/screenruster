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
use std::process::{Child, Command, Stdio};
use std::thread;
use std::sync::mpsc::{Receiver, TryRecvError, Sender, channel};

use toml;
use log;
pub use api::{Request, Response, Password, Pointer};

use api::json;
use error;

pub struct Saver {
	process:  Child,
	receiver: Receiver<Response>,
	sender:   Sender<Option<Request>>,

	starting: bool,
	stopping: bool,
	crashed:  bool,
}

impl Saver {
	/// Spawn a saver with the given name.
	pub fn spawn<S: AsRef<str>>(name: S) -> error::Result<Saver> {
		let mut child = Command::new(format!("screenruster-saver-{}", name.as_ref()))
			.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
			.spawn()?;

		let (sender, i_receiver) = channel();
		let (i_sender, receiver) = channel();

		// Reader.
		{
			let input = child.stdout.take().unwrap();
			let guard = i_sender.clone();

			thread::spawn(move || {
				for line in BufReader::new(input).lines() {
					if line.is_err() {
						break;
					}

					if let Ok(message) = json::parse(&line.unwrap()) {
						sender.send(match json!(message["type"].as_str()) {
							"initialized" => {
								Response::Initialized
							}

							"started" => {
								Response::Started
							}

							"stopped" => {
								Response::Stopped
							}

							_ =>
								continue
						}).unwrap();
					}
				}

				guard.send(None).unwrap();
			});
		}

		// Writer.
		{
			let mut output = child.stdin.take().unwrap();

			thread::spawn(move || {
				while let Ok(request) = receiver.recv() {
					if request.is_none() {
						break;
					}

					output.write_all(json::stringify(match request.unwrap() {
						Request::Config(config) => object!{
							"type"   => "config",
							"config" => config
						},

						Request::Target { display, screen, window } => object!{
							"type"    => "target",
							"display" => display,
							"screen"  => screen,
							"window"  => window
						},

						Request::Resize { width, height } => object!{
							"type"   => "resize",
							"width"  => width,
							"height" => height
						},

						Request::Pointer(Pointer::Move { x, y }) => object!{
							"type" => "pointer",
							"move" => object!{
								"x" => x,
								"y" => y
							}
						},

						Request::Pointer(Pointer::Button { x, y, button, press }) => object!{
							"type"   => "pointer",
							"button" => object!{
								"x"      => x,
								"y"      => y,
								"button" => button,
								"press"  => press
							}
						},

						Request::Password(password) => object!{
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

						Request::Start => object!{
							"type" => "start"
						},

						Request::Stop => object!{
							"type" => "stop"
						},
					}).as_bytes()).unwrap();

					output.write_all(b"\n").unwrap();
				}
			});
		}

		// Logger.
		{
			let input = child.stderr.take().unwrap();

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
			receiver: i_receiver,
			sender:   i_sender,

			crashed:  false,
			starting: false,
			stopping: false,
		})
	}

	pub fn is_crashed(&self) -> bool {
		self.crashed
	}

	pub fn is_starting(&self) -> bool {
		self.starting
	}

	pub fn is_stopping(&self) -> bool {
		self.stopping
	}

	pub fn kill(&mut self) {
		self.process.kill().unwrap();
		self.crashed = true;
	}

	/// Try to receive a message from the saver.
	pub fn recv(&mut self) -> Option<Response> {
		match self.receiver.try_recv() {
			Ok(response) =>
				Some(response),

			Err(TryRecvError::Empty) =>
				None,

			Err(TryRecvError::Disconnected) => {
				self.crashed = true;
				None
			}
		}
	}

	fn send(&mut self, request: Request) {
		if self.sender.send(Some(request)).is_err() {
			self.crashed = true;
		}
	}

	/// Configure the saver.
	pub fn config(&mut self, config: toml::Table) {
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

		self.send(Request::Config(convert(&toml::Value::Table(config))))
	}

	/// Select the rendering target for the saver.
	pub fn target<S: AsRef<str>>(&mut self, display: S, screen: i32, window: u64) {
		self.send(Request::Target {
			display: display.as_ref().into(),
			screen:  screen,
			window:  window,
		})
	}

	pub fn resize(&mut self, width: u32, height: u32) {
		self.send(Request::Resize {
			width:  width,
			height: height,
		})
	}

	pub fn pointer(&mut self, pointer: Pointer) {
		self.send(Request::Pointer(pointer))
	}

	pub fn password(&mut self, password: Password) {
		self.send(Request::Password(password))
	}

	/// Start the saver.
	pub fn start(&mut self) {
		self.starting = true;
		self.send(Request::Start)
	}

	/// Stop the saver.
	pub fn stop(&mut self) {
		self.stopping = true;
		self.send(Request::Stop)
	}
}
