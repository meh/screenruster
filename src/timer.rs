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
use std::ops::Deref;
use std::sync::mpsc::{Receiver, Sender, SendError, channel};
use std::time::{Instant, SystemTime, Duration};

use error;
use config;

pub struct Timer {
	receiver: Receiver<Response>,
	sender:   Sender<Request>,
}

#[derive(Clone, Debug)]
pub enum Request {
	/// Reset the specific event.
	Reset(Event),

	/// Requests a report on all the timers.
	Report {
		id: u64,
	},

	/// Suspend the timers.
	Suspend(SystemTime),

	/// Resume the timers.
	Resume,

	/// The screen was blanked.
	Blanked,

	/// The screen was unblanked.
	Unblanked,

	/// The screen saver was started.
	Started,

	/// The screen was locked.
	Locked,

	/// The screen saver was stopped, restarts all timers.
	Stopped,
}

#[derive(Clone, Debug)]
pub enum Event {
	Idle,
	Blank,
}

#[derive(Clone, Debug)]
pub enum Response {
	Report {
		id:        u64,
		beat:      Instant,
		idle:      Instant,
		started:   Option<Instant>,
		locked:    Option<Instant>,
		blanked:   Option<Instant>,
		unblanked: Option<Instant>,
	},

	Suspended(SystemTime),
	Resumed,

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

			// Instant to check when the screen saver starter.
			let mut started = None: Option<Instant>;

			// Instant to check when the screen was locked.
			let mut locked = None: Option<Instant>;

			// Instant to check when the screen was blanked.
			let mut blanked = None: Option<Instant>;

			// Instant to check when the screen was unblanked.
			let mut unblanked = None: Option<Instant>;

			// Instant to check when the timer was suspended.
			let mut suspended = None: Option<SystemTime>;

			// Time correction for suspension.
			let mut correction = 0;

			// Whether a correction loop has already been done.
			let mut corrected = false;

			loop {
				thread::sleep(Duration::from_secs(1));

				while let Ok(request) = receiver.try_recv() {
					match request {
						Request::Reset(Event::Idle) => {
							idle       = Instant::now();
							correction = 0;
						}

						Request::Reset(Event::Blank) | Request::Unblanked => {
							blanked   = None;
							unblanked = Some(Instant::now());
						}

						Request::Report { id } => {
							sender.send(Response::Report {
								id:        id,
								beat:      beat,
								idle:      idle,
								started:   started,
								locked:    locked,
								blanked:   blanked,
								unblanked: unblanked,
							}).unwrap();
						}

						Request::Suspend(time) => {
							suspended = Some(time);
							sender.send(Response::Suspended(time)).unwrap();
						}

						Request::Resume => {
							correction += suspended.take().unwrap().elapsed().unwrap_or(Duration::from_secs(0)).as_secs();
							corrected   = false;
						}

						Request::Blanked => {
							blanked = Some(Instant::now());
						}

						Request::Started => {
							started = Some(Instant::now());
						}

						Request::Locked => {
							locked = Some(Instant::now());
						}

						Request::Stopped => {
							idle       = Instant::now();
							started    = None;
							locked     = None;
							blanked    = None;
							correction = 0;
						}
					}
				}

				// If it's time to send a heart beat, send one and reset.
				if beat.elapsed().as_secs() >= config.beat as u64 {
					beat = Instant::now();
					sender.send(Response::Heartbeat).unwrap();
				}

				// Do not check events if the timers are suspended.
				if suspended.is_some() {
					continue;
				}

				// If blanking is enabled and the screen is not already blanked.
				if let (Some(after), false) = (config.blank, blanked.is_some()) {
					if unblanked.unwrap_or(idle).elapsed().as_secs() >= after as u64 {
						sender.send(Response::Blank).unwrap();
						blanked = Some(Instant::now());
					}
				}

				// If the system has been idle long enough send the message.
				if started.is_none() && idle.elapsed().as_secs() + correction >= config.timeout as u64 {
					sender.send(Response::Start).unwrap();
					started = Some(Instant::now());
				}

				// If the screen saver has been started, the screen is not locked and locking is enabled.
				if let (Some(start), Some(after), false) = (started, config.lock, locked.is_some()) {
					if start.elapsed().as_secs() + correction >= after as u64 {
						sender.send(Response::Lock).unwrap();
						locked = Some(Instant::now());
					}
				}

				// Only resume after one corrected loop, this avoids activities right
				// after resume cancelling timer events.
				if !corrected {
					sender.send(Response::Resumed).unwrap();
					corrected = true;
				}
			}
		});

		Ok(Timer {
			receiver: i_receiver,
			sender:   i_sender,
		})
	}

	pub fn reset(&self, event: Event) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Reset(event))
	}

	pub fn report(&self, id: u64) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Report { id: id })
	}

	pub fn suspend(&self, value: SystemTime) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Suspend(value))
	}

	pub fn resume(&self) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Resume)
	}

	pub fn blanked(&self) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Blanked)
	}

	pub fn unblanked(&self) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Unblanked)
	}

	pub fn started(&self) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Started)
	}

	pub fn locked(&self) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Locked)
	}

	pub fn stopped(&self) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Stopped)
	}
}

impl Deref for Timer {
	type Target = Receiver<Response>;

	fn deref(&self) -> &Receiver<Response> {
		&self.receiver
	}
}
