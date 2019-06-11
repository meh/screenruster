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

use channel::{self, Receiver, Sender, select};
use xcb;
use xkb::{self, key};

use crate::error;
use crate::config::Config;
use crate::saver::{self, Saver};
use api::{self, Password, Pointer};
use super::{Window};
use crate::platform::{self, Display, Keyboard};

pub struct Preview {
	receiver: Receiver<Response>,
	sender:   Sender<Request>,
}

#[derive(Clone)]
pub enum Request {

}

#[derive(Clone)]
pub enum Response {
	Done,
}

impl Preview {
	pub fn spawn<T: AsRef<str>>(name: T, config: Config) -> error::Result<Preview> {
		let     display  = Display::open(None)?;
		let mut keyboard = Keyboard::new(display.clone(), None)?;
		let     window   = Window::create(display.clone())?;
		let mut saver    = Saver::spawn(name.as_ref())?;
		let mut throttle = config.saver().throttle();

		saver.config(config.saver().get(name)).unwrap();
		saver.target(display.name(), window.screen(), window.id() as u64).unwrap();

		if throttle {
			saver.throttle(true).unwrap();
		}

		let (sender, i_receiver) = channel::unbounded();
		let (i_sender, receiver) = channel::unbounded();

		thread::spawn(move || {
			let x = platform::display::sink(&display);
			let s = saver.take().unwrap();

			loop {
				select! {
					// Handle control events.
					recv(receiver) -> event => {
						match event {
							_ => ()
						}
					},

					// Handle saver events.
					recv(s) -> event => {
						match event.unwrap() {
							saver::Response::Forward(api::Response::Initialized) => {
								saver.start().unwrap();
							}

							saver::Response::Forward(api::Response::Started) => {
								if saver.was_started() {
									window.show();
								}
								else {
									saver.kill();
								}
							}

							saver::Response::Forward(api::Response::Stopped) => {
								if !saver.was_stopped() {
									saver.kill();
								}
							}

							saver::Response::Exit(..) => {
								break;
							}
						}
					},

					// Handle X events.
					recv(x) -> event => {
						let event = event.unwrap();

						match event.response_type() {
							// Handle keyboard events.
							e if keyboard.owns_event(e) => {
								keyboard.handle(&event);
							}

							xcb::CONFIGURE_NOTIFY => {
								let event = unsafe { xcb::cast_event::<xcb::ConfigureNotifyEvent>(&event) };
								saver.resize(event.width() as u32, event.height() as u32).unwrap();
							}

							// Handle keyboard input.
							xcb::KEY_PRESS => {
								let key = unsafe { xcb::cast_event::<xcb::KeyPressEvent>(&event) };

								match keyboard.symbol(key.detail().into()) {
									// Toggle throttling.
									Some(key::t) | Some(key::T) => {
										throttle = !throttle;
										saver.throttle(throttle).unwrap();
									}

									// Stop the preview.
									Some(key::q) | Some(key::Q) => {
										saver.stop().unwrap();
									}

									// Test password insertion.
									Some(key::i) | Some(key::I) => {
										saver.password(Password::Insert).unwrap();
									}

									// Test password deletetion.
									Some(key::d) | Some(key::D) => {
										saver.password(Password::Delete).unwrap();
									}

									// Test passsword reset.
									Some(key::r) | Some(key::R) => {
										saver.password(Password::Reset).unwrap();
									}

									// Test password check.
									Some(key::c) | Some(key::C) => {
										saver.password(Password::Check).unwrap();
									}

									// Test password success.
									Some(key::s) | Some(key::S) => {
										saver.password(Password::Success).unwrap();
									}

									// Test password failure.
									Some(key::f) | Some(key::F) => {
										saver.password(Password::Failure).unwrap();
									}

									_ => ()
								}
							}

							// Handle mouse button presses.
							xcb::BUTTON_PRESS | xcb::BUTTON_RELEASE => {
								let event = unsafe { xcb::cast_event::<xcb::ButtonPressEvent>(&event) };

								saver.pointer(Pointer::Button {
									x: event.event_x() as i32,
									y: event.event_y() as i32,

									button: event.detail(),
									press:  event.response_type() == xcb::BUTTON_PRESS,
								}).unwrap();
							}

							// Handle mouse motion.
							xcb::MOTION_NOTIFY => {
								let event = unsafe { xcb::cast_event::<xcb::MotionNotifyEvent>(&event) };

								saver.pointer(Pointer::Move {
									x: event.event_x() as i32,
									y: event.event_y() as i32,
								}).unwrap();
							}

							_ => ()
						}
					}
				}
			}

			sender.send(Response::Done).unwrap();
		});

		Ok(Preview {
			receiver: i_receiver,
			sender:   i_sender,
		})
	}
}

impl Deref for Preview {
	type Target = Receiver<Response>;

	fn deref(&self) -> &Receiver<Response> {
		&self.receiver
	}
}
