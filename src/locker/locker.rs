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

use std::collections::HashMap;
use std::thread;
use std::ops::Deref;
use std::sync::mpsc::{Receiver, Sender, SendError, channel};

use rand::{self, Rng};
use xcb;
use xkbcommon::xkb;
use xkbcommon::xkb::keysyms as key;

use error;
use config::Config;
use api;
use timer;
use saver::{self, Saver, Safety, Password, Pointer};
use super::{Display, Window};
use platform::Keyboard;

pub struct Locker {
	receiver: Receiver<Response>,
	sender:   Sender<Request>,
}

#[derive(Clone)]
pub enum Request {
	Sanitize,
	Timeout { id: u64 },
	Activity,
	Power(bool),
	Throttle(bool),

	Start,
	Lock,
	Auth(bool),
	Stop,
}

#[derive(Clone)]
pub enum Response {
	Timeout(timer::Timeout),
	Activity,
	Password(String),
	Stopped,
}

impl Locker {
	pub fn spawn(config: Config) -> error::Result<Locker> {
		let     display  = Display::open(config.locker())?;
		let mut keyboard = Keyboard::new((*display).clone())?;
		let mut windows  = HashMap::new(): HashMap<u32, Window>;
		let mut savers   = HashMap::new(): HashMap<u32, Saver>;
		let mut checking = false;
		let mut password = String::new();

		for screen in 0 .. display.screens() {
			let window = Window::create(display.clone(), screen as i32)?;

			display.observe(window.root());
			windows.insert(window.id(), window);
		}

		let (sender,   i_receiver)   = channel();
		let (i_sender, receiver)   = channel();
		let (s_sender, s_receiver) = channel();

		thread::spawn(move || {
			macro_rules! window {
				(? $id:expr) => (
					windows.get_mut(&$id)
				);

				($id:expr) => (
					windows.get_mut(&$id).unwrap()
				);
			}

			macro_rules! saver {
				(? $id:expr) => (
					savers.get_mut(&$id)
				);

				($id:expr) => (
					savers.get_mut(&$id).unwrap()
				);

				(safety $id:expr) => (
					saver!(safety on window!($id));
				);

				(safety on $window:expr) => (
					if let Some(saver) = saver!(? $window.id()) {
						if $window.has_keyboard() && $window.has_pointer() {
							saver.safety(Safety::High).unwrap();
						}
						else if $window.has_keyboard() {
							saver.safety(Safety::Medium).unwrap();
						}
						else {
							saver.safety(Safety::Low).unwrap();
						}
					}
				)
			}

			let x = (***display).as_ref();

			loop {
				select! {
					// Handle control events.
					event = receiver.recv() => {
						match event.unwrap() {
							Request::Timeout { id } => {
								saver!(id as u32).kill();
							}

							Request::Sanitize => {
								display.sanitize();

								for window in windows.values_mut() {
									let keyboard = window.has_keyboard();
									let pointer  = window.has_pointer();

									window.sanitize();

									if keyboard == window.has_keyboard() && pointer == window.has_pointer() {
										continue;
									}

									saver!(safety on window);
								}
							}

							Request::Activity => {
								sender.send(Response::Activity).unwrap();
							}

							Request::Throttle(value) => {
								for saver in savers.values_mut() {
									saver.throttle(value).unwrap();
								}
							}

							Request::Power(value) => {
								for window in windows.values_mut() {
									window.power(value);
								}

								for saver in savers.values_mut() {
									saver.blank(!value).unwrap();
								}

								display.power(value);
							}

							Request::Start => {
								for window in windows.values_mut() {
									if !config.saver().using().is_empty() {
										let name = &config.saver().using()[rand::thread_rng().gen_range(0, config.saver().using().len())];

										if let Ok(mut saver) = Saver::spawn(&name) {
											let id = window.id();

											sender.send(Response::Timeout(timer::Timeout::Set {
												id:      id as u64,
												seconds: config.saver().timeout() as u64,
											})).unwrap();

											let receiver = saver.take().unwrap();
											let sender   = s_sender.clone();

											thread::spawn(move || {
												while let Ok(event) = receiver.recv() {
													sender.send((id, event)).unwrap();
												}
											});

											saver.config(config.saver().get(&name)).unwrap();
											saver.target(display.name(), window.screen(), id as u64).unwrap();

											if config.saver().throttle() {
												saver.throttle(true).unwrap();
											}

											savers.insert(id, saver);

											continue;
										}
									}

									window.lock().unwrap();
									window.blank();
								}
							}

							Request::Lock => {
								for saver in savers.values_mut() {
									saver.lock().unwrap();
								}
							}

							Request::Auth(state) => {
								checking = false;

								for saver in savers.values_mut() {
									saver.password(if state { Password::Success } else { Password::Failure }).unwrap();
								}
							}

							Request::Stop => {
								for (&id, window) in &mut windows {
									if let Some(saver) = savers.get_mut(&id) {
										sender.send(Response::Timeout(timer::Timeout::Set {
											id:      id as u64,
											seconds: config.saver().timeout() as u64,
										})).unwrap();

										saver.stop().unwrap();
									}
									else {
										window.unlock().unwrap();
									}
								}
							}
						}
					},

					// Handle saver events.
					event = s_receiver.recv() => {
						let (id, event) = event.unwrap();

						match event {
							saver::Response::Forward(api::Response::Initialized) => {
								saver!(id).start().unwrap();
							}

							saver::Response::Forward(api::Response::Started) => {
								if saver!(id).was_started() {
									sender.send(Response::Timeout(timer::Timeout::Cancel { id: id as u64 })).unwrap();

									window!(id).lock().unwrap();
									saver!(safety id);
								}
								else {
									saver!(id).kill();
								}
							}

							saver::Response::Forward(api::Response::Stopped) => {
								if !saver!(id).was_stopped() {
									saver!(id).kill();
								}
								else {
									sender.send(Response::Timeout(timer::Timeout::Cancel { id: id as u64 })).unwrap();
								}
							}

							saver::Response::Exit(..) => {
								if saver!(id).was_stopped() {
									window!(id).unlock().unwrap();

									if savers.len() == 1 {
										sender.send(Response::Stopped).unwrap();
									}
								}
								else {
									window!(id).lock().unwrap();
									window!(id).blank();
								}

								savers.remove(&id);
							}
						}
					},

					// Handle X events.
					event = x.recv() => {
						let event = event.unwrap();

						match event.response_type() {
							// Handle screen changes.
							e if display.randr().map_or(false, |rr| e == rr.first_event() + xcb::randr::SCREEN_CHANGE_NOTIFY) => {
								let event = xcb::cast_event(&event): &xcb::randr::ScreenChangeNotifyEvent;

								for window in windows.values_mut() {
									if window.root() == event.root() {
										window.resize(event.width() as u32, event.height() as u32);

										if let Some(saver) = savers.get_mut(&window.id()) {
											saver.resize(event.width() as u32, event.height() as u32).unwrap();
										}
									}
								}
							}

							// Handle keyboard events.
							e if keyboard.owns_event(e) => {
								keyboard.handle(&event);
							}

							// Handle keyboard input.
							//
							// Note we only act on key presses because `Xutf8LookupString`
							// only generates strings from `KeyPress` events.
							xcb::KEY_PRESS => {
								sender.send(Response::Activity).unwrap();

								// Ignore keyboard input while checking authentication.
								if checking {
									continue;
								}

								let event = xcb::cast_event(&event): &xcb::KeyPressEvent;
								if let Some(_window) = windows.values().find(|w| w.id() == event.event()) {
									match keyboard.symbol(event.detail() as xkb::Keycode) {
										// Delete a character.
										key::KEY_BackSpace => {
											password.pop();

											for saver in savers.values_mut() {
												saver.password(Password::Delete).unwrap();
											}
										}

										// Clear the password.
										key::KEY_Escape => {
											password.clear();

											for saver in savers.values_mut() {
												saver.password(Password::Reset).unwrap();
											}
										}

										// Check authentication.
										key::KEY_Return => {
											for saver in savers.values_mut() {
												saver.password(Password::Check).unwrap();
											}

											sender.send(Response::Password(password)).unwrap();

											checking = true;
											password = String::new();
										}

										_ => {
											// Limit the maximum password length so keeping a button
											// pressed is not going to OOM us in the extremely long
											// run.
											if password.len() <= 255 {
												for ch in keyboard.string(event.detail() as xkb::Keycode).chars() {
													password.push(ch);

													for saver in savers.values_mut() {
														saver.password(Password::Insert).unwrap();
													}
												}
											}
										}
									}
								}
							}

							xcb::KEY_RELEASE => {
								sender.send(Response::Activity).unwrap();
							}

							// Handle mouse button presses.
							xcb::BUTTON_PRESS | xcb::BUTTON_RELEASE => {
								sender.send(Response::Activity).unwrap();

								let event = xcb::cast_event(&event): &xcb::ButtonPressEvent;
								if let Some(window) = windows.values().find(|w| w.id() == event.event()) {
									if let Some(saver) = savers.get_mut(&window.id()) {
										saver.pointer(Pointer::Button {
											x: event.event_x() as i32,
											y: event.event_y() as i32,

											button: event.detail(),
											press:  event.response_type() == xcb::BUTTON_PRESS,
										}).unwrap()
									}
								}
							}

							// Handle mouse motion.
							xcb::MOTION_NOTIFY => {
								sender.send(Response::Activity).unwrap();

								let event = xcb::cast_event(&event): &xcb::MotionNotifyEvent;
								if let Some(window) = windows.values().find(|w| w.id() == event.event()) {
									if let Some(saver) = savers.get_mut(&window.id()) {
										saver.pointer(Pointer::Move {
											x: event.event_x() as i32,
											y: event.event_y() as i32,
										}).unwrap();
									}
								}
							}

							// On window changes, try to observe the window.
							xcb::MAP_NOTIFY | xcb::CONFIGURE_NOTIFY => {
								let event = xcb::cast_event(&event): &xcb::MapNotifyEvent;
								display.observe(event.window());
							}

							_ => ()
						}
					}
				}
			}
		});

		Ok(Locker {
			receiver: i_receiver,
			sender:   i_sender,
		})
	}

	pub fn sanitize(&self) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Sanitize)
	}

	pub fn timeout(&self, id: u64) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Timeout { id: id })
	}

	pub fn start(&self) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Start)
	}

	pub fn lock(&self) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Lock)
	}

	pub fn auth(&self, value: bool) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Auth(value))
	}

	pub fn stop(&self) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Stop)
	}

	pub fn power(&self, value: bool) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Power(value))
	}

	pub fn activity(&self) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Activity)
	}

	pub fn throttle(&self, value: bool) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Throttle(value))
	}
}

impl Deref for Locker {
	type Target = Receiver<Response>;

	fn deref(&self) -> &Receiver<Response> {
		&self.receiver
	}
}
