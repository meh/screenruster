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
	Reset(Event),
	Restart,
	Report,
}

#[derive(Clone, Debug)]
pub enum Event {
	Idle,
	Blank,
}

#[derive(Clone, Debug)]
pub enum Response {
	Heartbeat,

	Start,
	Lock,
	Blank,

	Report {
		beat:      Instant,
		idle:      Instant,
		started:   Option<Instant>,
		locked:    Option<Instant>,
		blanked:   Option<Instant>,
		unblanked: Option<Instant>,
	}
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

			// Instant to check when the screen saver starter.
			let mut started = None: Option<Instant>;

			// Instant to check when the screen was locked.
			let mut locked = None: Option<Instant>;

			// Instant to check when the screen was blanked.
			let mut blanked = None: Option<Instant>;

			// Instant to check when the screen was unblanked.
			let mut unblanked = None: Option<Instant>;

			loop {
				thread::sleep(Duration::from_secs(1));

				while let Ok(request) = receiver.try_recv() {
					match request {
						Request::Reset(Event::Idle) => {
							// If the saver has not started refresh the idle time.
							if started.is_none() {
								idle = Instant::now();
							}
						}

						Request::Reset(Event::Blank) => {
							if started.is_some() {
								blanked   = None;
								unblanked = Some(Instant::now());
							}
						}

						Request::Restart => {
							// If the saver was started reset guards to initial state.
							if started.is_some() {
								idle    = Instant::now();
								started = None;
								locked  = None;
								blanked = None;
							}
						}

						Request::Report => {
							sender.send(Response::Report {
								beat:      beat,
								idle:      idle,
								started:   started,
								locked:    locked,
								blanked:   blanked,
								unblanked: unblanked,
							}).unwrap();
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
								sender.send(Response::Lock).unwrap();
							}
						}
					}

					// If the screen is not blanked.
					if blanked.is_none() {
						// If blanking is enabled.
						if let Some(after) = config.blank {
							// If it's time to blank, send th emessage and enable the blank guard.
							if unblanked.unwrap_or(start).elapsed().as_secs() >= after as u64 {
								blanked = Some(Instant::now());
								sender.send(Response::Blank).unwrap();
							}
						}
					}
				}
				else {
					// If the system has been idle long enough send the message.
					if idle.elapsed().as_secs() >= config.timeout as u64 {
						sender.send(Response::Start).unwrap();
						started = Some(Instant::now());
					}
				}
			}
		});

		Ok(Timer {
			receiver: i_receiver,
			sender:   i_sender,
		})
	}

	pub fn reset(&self, event: Event) {
		self.sender.send(Request::Reset(event)).unwrap();
	}

	pub fn restart(&self) {
		self.sender.send(Request::Restart).unwrap();
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
