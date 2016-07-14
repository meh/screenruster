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

use std::ptr;
use std::mem;
use std::str;
use std::collections::{HashSet, HashMap};
use std::thread;
use std::time::Duration;
use std::ops::Deref;
use std::sync::mpsc::{Receiver, Sender, SendError, channel};
use std::sync::Arc;

use rand::{self, Rng};
use x11::{xlib, keysym, xrandr};
use libc::{c_int, c_uint};

use error;
use config::Config;
use api;
use saver::{self, Saver, Password, Pointer};
use super::{Display, Window};

/// The actual locker.
///
/// Deals with ugly X11 shit and handles savers.
///
/// TODO(meh): Add a timeout to stopping a saver, if it takes too long it may
///            be stuck or trying to ruse us.
///
/// TODO(meh): Consider timeouts for other saver commands.
#[derive(Debug)]
pub struct Locker {
	receiver: Receiver<Response>,
	sender:   Sender<Request>,
	display:  Arc<Display>,
}

#[derive(Clone)]
pub enum Request {
	Sanitize,
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
	Activity,
	Password(String),
}

impl Locker {
	pub fn spawn(config: Config) -> error::Result<Locker> {
		unsafe {
			let mut windows  = HashMap::new(): HashMap<xlib::Window, Window>;
			let mut savers   = HashMap::new(): HashMap<xlib::Window, Saver>;
			let mut starting = false;
			let mut stopping = false;
			let mut checking = false;
			let mut password = String::new();
			let mut event    = mem::zeroed(): xlib::XEvent;

			let display = Display::open(config.locker())?;
			display.sanitize();

			for screen in 0 .. xlib::XScreenCount(display.id) {
				let window = Window::create(display.clone(), screen)?;

				display.observe(window.root);
				windows.insert(window.id, window);
			}

			let (sender, i_receiver) = channel();
			let (i_sender, receiver) = channel();

			// FIXME(meh): The whole `XPending` check and sleeping for 100ms to then
			//             `try_recv` on the channels is very fragile.
			//
			//             Find a better way to do it that plays well with Xlib's
			//             threading model (yeah, right, guess moving to XCB would be
			//             the right way, or reimplementing it properly for Rust).
			{
				let display = display.clone();

				thread::spawn(move || {
					loop {
						// Check if there are any control messages.
						if let Ok(message) = receiver.try_recv() {
							match message {
								Request::Sanitize => {
									display.sanitize();

									for window in windows.values_mut() {
										window.sanitize();
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
									for saver in savers.values_mut() {
										saver.blank(!value).unwrap();
									}

									display.power(value);
								}

								Request::Start => {
									starting = true;
									stopping = false;

									for window in windows.values_mut() {
										if !config.saver().using().is_empty() {
											let name = config.saver().using()[rand::thread_rng().gen_range(0, config.saver().using().len())];

											if let Ok(mut saver) = Saver::spawn(name) {
												saver.config(config.saver().get(name)).unwrap();
												saver.target(display.name(), window.screen, window.id).unwrap();

												if config.saver().throttle() {
													saver.throttle(true).unwrap();
												}

												savers.insert(window.id, saver);
												continue;
											}
										}

										// FIXME(meh): Do not crash on grab failure.
										window.lock().unwrap();
										window.blank();
									}
								}

								Request::Lock => {
									password = String::new();
								}

								Request::Auth(state) => {
									checking = false;

									for saver in savers.values_mut() {
										saver.password(if state { Password::Success } else { Password::Failure }).unwrap();
									}
								}

								Request::Stop => {
									starting = false;
									stopping = true;

									for window in windows.values_mut() {
										if let Some(saver) = savers.get_mut(&window.id) {
											saver.stop().unwrap();
										}
										else {
											window.unlock();
										}
									}
								}
							}

							continue;
						}

						// Check if there are any messages from savers.
						{
							let mut stopped  = HashSet::new();
							let mut returned = HashSet::new();

							for (&id, saver) in &mut savers {
								match saver.recv() {
									Some(saver::Response::Forward(api::Response::Initialized)) => {
										saver.start().unwrap();
									}

									Some(saver::Response::Forward(api::Response::Started)) => {
										if saver.was_started() {
											// FIXME(meh): Do not crash on grab failure.
											windows.get_mut(&id).unwrap().lock().unwrap();
										}
										else {
											saver.kill();
										}
									}

									Some(saver::Response::Forward(api::Response::Stopped)) => {
										if saver.was_stopped() {
											stopped.insert(id);
										}
										else {
											saver.kill();
										}
									}

									Some(saver::Response::Exit(..)) => {
										returned.insert(id);
									}

									None => (),
								}
							}

							for id in &returned {
								stopped.remove(id);

								let saver  = savers.remove(id).unwrap();
								let window = windows.get_mut(id).unwrap();

								if !saver.was_stopped() {
									window.lock().unwrap();
									window.blank();
								}
								else {
									window.unlock();
								}
							}

							// Unlock the stopped savers.
							for id in &stopped {
								windows.get_mut(id).unwrap().unlock();
								savers.remove(id);
							}
						}

						// Check if there are any pending events, or sleep 100ms.
						if xlib::XPending(display.id) == 0 {
							thread::sleep(Duration::from_millis(100));
							continue;
						}

						xlib::XNextEvent(display.id, &mut event);
						let any = xlib::XAnyEvent::from(event);

						match event.get_type() {
							// Handle screen changes.
							e if display.randr.as_ref().map_or(false, |rr| e == rr.event(xrandr::RRScreenChangeNotify)) => {
								let event = xrandr::XRRScreenChangeNotifyEvent::from(event);

								for window in windows.values_mut() {
									if window.root == event.root {
										window.resize(event.width as u32, event.height as u32);

										if let Some(saver) = savers.get_mut(&window.id) {
											saver.resize(event.width as u32, event.height as u32).unwrap();
										}
									}
								}
							}

							// Handle keyboard input.
							//
							// Note we only act on key presses because `Xutf8LookupString`
							// only generates strings from `KeyPress` events.
							xlib::KeyPress => {
								sender.send(Response::Activity).unwrap();

								// Ignore keyboard input while checking authentication.
								if checking {
									continue;
								}

								if let Some(window) = windows.values().find(|w| w.id == any.window) {
									let mut key  = xlib::XKeyEvent::from(event);
									let     code = key.keycode;

									match xlib::XKeycodeToKeysym(window.display.id, code as xlib::KeyCode, 0) as c_uint {
										// Delete a character.
										keysym::XK_BackSpace => {
											password.pop();

											for saver in savers.values_mut() {
												saver.password(Password::Delete).unwrap();
											}
										}

										// Clear the password.
										keysym::XK_Escape => {
											password.clear();

											for saver in savers.values_mut() {
												saver.password(Password::Reset).unwrap();
											}
										}

										// Check authentication.
										keysym::XK_Return => {
											checking = true;

											for saver in savers.values_mut() {
												saver.password(Password::Check).unwrap();
											}

											sender.send(Response::Password(password)).unwrap();
											password = String::new();
										}

										_ => {
											let mut ic_sym = 0;
											let mut buffer = [0u8; 16];
											let     count  = xlib::Xutf8LookupString(window.ic, &mut key,
												mem::transmute(buffer.as_mut_ptr()), buffer.len() as c_int,
												&mut ic_sym, ptr::null_mut());

											for ch in str::from_utf8(&buffer[..count as usize]).unwrap_or("").chars() {
												password.push(ch);

												for saver in savers.values_mut() {
													saver.password(Password::Insert).unwrap();
												}
											}
										}
									}
								}
							}

							xlib::KeyRelease => {
								sender.send(Response::Activity).unwrap();
							}

							// Handle mouse button presses.
							xlib::ButtonPress | xlib::ButtonRelease => {
								sender.send(Response::Activity).unwrap();

								if let Some(window) = windows.values().find(|w| w.id == any.window) {
									if let Some(saver) = savers.get_mut(&window.id) {
										let event = xlib::XButtonEvent::from(event);

										saver.pointer(Pointer::Button {
											x: event.x,
											y: event.y,

											button: event.button as u8,
											press:  event.type_ == xlib::ButtonPress,
										}).unwrap()
									}
								}
							}

							// Handle mouse motion.
							xlib::MotionNotify => {
								sender.send(Response::Activity).unwrap();

								if let Some(window) = windows.values().find(|w| w.id == any.window) {
									if let Some(saver) = savers.get_mut(&window.id) {
										let event = xlib::XMotionEvent::from(event);

										saver.pointer(Pointer::Move {
											x: event.x,
											y: event.y,
										}).unwrap();
									}
								}
							}

							// On window changes, try to observe the window.
							xlib::MapNotify | xlib::ConfigureNotify => {
								display.observe(any.window);
							}

							_ => ()
						}
					}
				});
			}

			Ok(Locker {
				receiver: i_receiver,
				sender:   i_sender,
				display:  display,
			})
		}
	}

	pub fn sanitize(&self) -> Result<(), SendError<Request>> {
		self.sender.send(Request::Sanitize)
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
