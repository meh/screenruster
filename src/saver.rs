use std::io::{self, BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::sync::mpsc::{Receiver, RecvError, TryRecvError, Sender, SendError, channel};

use toml;
use log;
pub use api::{Request, Response, Password};

use api::json;
use error;

#[derive(Debug)]
pub struct Saver {
	receiver: Receiver<Response>,
	sender:   Sender<Option<Request>>,
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
						debug!("incoming: {:?}", message);

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

					debug!("outgoing: {:?}", request);

					output.write_all(json::stringify(match request.unwrap() {
						Request::Target { display, screen, window } => object!{
							"type"    => "target",
							"display" => display,
							"screen"  => screen,
							"window"  => window
						},

						Request::Config(config) => object!{
							"type"   => "config",
							"config" => config
						},

						Request::Dialog(active) => object!{
							"type"   => "dialog",
							"dialog" => active
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
			let mut input = child.stderr.take().unwrap();

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
			receiver: i_receiver,
			sender:   i_sender,
		})
	}

	/// Try to receive a message from the saver.
	pub fn recv(&self) -> Result<Option<Response>, RecvError> {
		match self.receiver.try_recv() {
			Ok(response) =>
				Ok(Some(response)),

			Err(TryRecvError::Empty) =>
				Ok(None),

			Err(TryRecvError::Disconnected) =>
				Err(RecvError)
		}
	}

	fn send(&self, request: Request) -> Result<(), SendError<Option<Request>>> {
		self.sender.send(Some(request))
	}

	/// Configure the saver.
	pub fn config(&self, config: toml::Table) -> Result<(), SendError<Option<Request>>> {
		fn convert(value: &toml::Value) -> json::JsonValue {
			match value {
				&toml::Value::String(ref value) =>
					value.clone().into(),

				&toml::Value::Integer(value) =>
					value.into(),

				&toml::Value::Float(value) =>
					value.into(),

				&toml::Value::Boolean(value) =>
					value.into(),

				&toml::Value::Datetime(ref value) =>
					value.clone().into(),

				&toml::Value::Array(ref value) =>
					json::JsonValue::Array(value.iter().map(|v| convert(v)).collect()),

				&toml::Value::Table(ref value) =>
					json::JsonValue::Object(value.iter().map(|(k, v)| (k.clone(), convert(v))).collect()),
			}
		}

		debug!("{:?}", json::stringify(object!{ "foo" => 0.0001 }));

		self.send(Request::Config(convert(&toml::Value::Table(config))))
	}

	/// Select the rendering target for the saver.
	pub fn target<S: AsRef<str>>(&self, display: S, screen: i32, window: u64) -> Result<(), SendError<Option<Request>>> {
		self.send(Request::Target {
			display: display.as_ref().into(),
			screen:  screen,
			window:  window,
		})
	}

	/// Start the saver.
	pub fn start(&self) -> Result<(), SendError<Option<Request>>> {
		self.send(Request::Start)
	}

	/// Stop the saver.
	pub fn stop(&self) -> Result<(), SendError<Option<Request>>> {
		self.send(Request::Stop)
	}
}
