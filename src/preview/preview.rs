use std::mem;
use std::thread;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, SendError, channel};
use std::ops::Deref;
use std::time::Duration;

use libc::c_uint;
use x11::{xlib, keysym};

use error;
use config::Config;
use saver::{self, Saver};
use api::{self, Password, Pointer};
use super::{Display, Window};

pub struct Preview {
	receiver: Receiver<Response>,
	sender:   Sender<Request>,
	display:  Arc<Display>,
}

#[derive(Clone)]
pub enum Request {

}

#[derive(Clone)]
pub enum Response {
	Done(Sender<Response>),
}

impl Preview {
	pub fn spawn<T: AsRef<str>>(name: T, config: Config) -> error::Result<Preview> {
		unsafe {
			let display = Display::open()?;
			let window  = Window::create(display.clone(), xlib::XDefaultScreen(display.id))?;

			let mut saver = Saver::spawn(name.as_ref())?;
			saver.config(config.saver().get(name)).unwrap();
			saver.target(display.name(), window.screen, window.id).unwrap();

			let mut throttle = config.saver().throttle();
			if throttle {
				saver.throttle(true).unwrap();
			}

			let (sender, i_receiver) = channel();
			let (i_sender, receiver) = channel();

			{
				let display = display.clone();

				thread::spawn(move || {
					let mut event = mem::zeroed();

					'event: loop {
						// Check if there are any control messages.
						while let Ok(message) = receiver.try_recv() {
							match message {
								_ => ()
							}
						}
	
						// Check if there are any messages from the saver.
						{
							while let Some(response) = saver.recv() {
								match response {
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
										break 'event;
									}
								}
							}
						}

						// Check if there are any pending events, or sleep 100ms.
						if xlib::XPending(display.id) == 0 {
							thread::sleep(Duration::from_millis(100));
							continue;
						}

						xlib::XNextEvent(display.id, &mut event);

						match event.get_type() {
							// Handle key presses.
							xlib::KeyPress => {
								let key  = xlib::XKeyEvent::from(event);
								let code = key.keycode;

								match xlib::XKeycodeToKeysym(window.display.id, code as xlib::KeyCode, 0) as c_uint {
									// Toggle throttling.
									keysym::XK_t | keysym::XK_T => {
										throttle = !throttle;
										saver.throttle(throttle).unwrap();
									}

									// Stop the preview.
									keysym::XK_q | keysym::XK_Q => {
										saver.stop().unwrap();
									}

									// Test password insertion.
									keysym::XK_i | keysym::XK_I => {
										saver.password(Password::Insert).unwrap();
									}

									// Test password deletetion.
									keysym::XK_d | keysym::XK_D => {
										saver.password(Password::Delete).unwrap();
									}

									// Test passsword reset.
									keysym::XK_r | keysym::XK_R => {
										saver.password(Password::Reset).unwrap();
									}

									// Test password check.
									keysym::XK_c | keysym::XK_C => {
										saver.password(Password::Check).unwrap();
									}

									// Test password success.
									keysym::XK_s | keysym::XK_S => {
										saver.password(Password::Success).unwrap();
									}

									// Test password failure.
									keysym::XK_f | keysym::XK_F => {
										saver.password(Password::Failure).unwrap();
									}

									_ => ()
								}
							}

							// Handle mouse buttons.
							xlib::ButtonPress | xlib::ButtonRelease => {
								let event = xlib::XButtonEvent::from(event);

								saver.pointer(Pointer::Button {
									x: event.x,
									y: event.y,

									button: event.button as u8,
									press:  event.type_ == xlib::ButtonPress,
								}).unwrap()
							}

							// Handle mouse movement.
							xlib::MotionNotify => {
								let event = xlib::XMotionEvent::from(event);

								saver.pointer(Pointer::Move {
									x: event.x,
									y: event.y,
								}).unwrap();
							}

							_ => ()
						}
					}

					sender.send(Response::Done(sender.clone())).unwrap();
				});
			}

			Ok(Preview {
				receiver: i_receiver,
				sender:   i_sender,
				display:  display,
			})
		}
	}
}

impl Deref for Preview {
	type Target = Receiver<Response>;

	fn deref(&self) -> &Receiver<Response> {
		&self.receiver
	}
}
