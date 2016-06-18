use std::thread;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Instant, Duration};

use error;
use config;

pub struct Timer {
	receiver: Receiver<Response>,
	sender:   Sender<Request>,
}

#[derive(Clone, Debug)]
pub enum Request {
	Activity,
	Started,
	Stopped,
}

#[derive(Clone, Debug)]
pub enum Response {
	Heartbeat,

	Start,
	Lock,
	Blank,
}

impl Timer {
	pub fn spawn(config: config::Timer) -> error::Result<Timer> {
		let (sender, i_receiver) = channel();
		let (i_sender, receiver) = channel();

		thread::spawn(move || {
			// Instant to check last heartbeat.
			let mut beat = Instant::now();

			// Instant to check last activity time.
			let mut idle = Instant::now();

			// Optional instant to check when the screen saver starter.
			let mut started = None: Option<Instant>;

			// Optional instant to check when the screen was locked.
			let mut locked = None: Option<Instant>;

			loop {
				thread::sleep(Duration::from_secs(1));

				while let Ok(request) = receiver.try_recv() {
					match request {
						Request::Activity => {
							// If the saver has not started refresh the idle time.
							if started.is_none() {
								idle = Instant::now();
							}
						}

						Request::Started => {
							// If the saver wasn't started already enable the guards.
							if started.is_none() {
								started = Some(Instant::now());
							}
						}

						Request::Stopped => {
							// If the saver was started reset guards to initial state.
							if started.is_some() {
								idle    = Instant::now();
								started = None;
								locked  = None;
							}
						}
					}
				}

				// If it's time to send a heart beat, send one and reset.
				if beat.elapsed().as_secs() >= config.beat as u64 {
					beat = Instant::now();
					sender.send(Response::Heartbeat).unwrap();
				}

				// If the screen saver has been started.
				if let Some(start) = started {
					// If the screen is not locked.
					if locked.is_none() {
						// If locking is enabled.
						if let Some(after) = config.lock {
							// If it's time to lock, send the message and enable the lock guard.
							if start.elapsed().as_secs() >= after as u64 {
								locked = Some(Instant::now());
								sender.send(Response::Lock);
							}
						}
					}
				}
				else {
					// If the system has been idle long enough send the message.
					if idle.elapsed().as_secs() >= config.timeout as u64 {
						sender.send(Response::Start).unwrap();
					}
				}
			}
		});

		Ok(Timer {
			receiver: i_receiver,
			sender:   i_sender,
		})
	}

	pub fn activity(&self) {
		self.sender.send(Request::Activity).unwrap();
	}

	pub fn started(&self) {
		self.sender.send(Request::Started).unwrap();
	}

	pub fn stopped(&self) {
		self.sender.send(Request::Stopped).unwrap();
	}
}

impl AsRef<Receiver<Response>> for Timer {
	fn as_ref(&self) -> &Receiver<Response> {
		&self.receiver
	}
}

impl AsRef<Sender<Request>> for Timer {
	fn as_ref(&self) -> &Sender<Request> {
		&self.sender
	}
}

