use std::ptr;
use std::mem;
use std::str;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use std::sync::mpsc::{Receiver, Sender, SendError, channel};
use std::sync::Arc;
use std::ffi::CStr;

use rand::{self, Rng};
use x11::{xlib, glx, keysym};
use libc::{c_int, c_uint, c_ulong};

use error;
use config::Config;
use util;
use saver::{self, Saver};

#[derive(Debug)]
pub struct Locker {
	receiver: Receiver<Response>,
	sender:   Sender<Request>,
	display:  Arc<Display>,
}

#[derive(Debug)]
pub struct Display {
	pub id: *mut xlib::Display,
}

unsafe impl Send for Display { }
unsafe impl Sync for Display { }

#[derive(Debug)]
pub struct Window {
	pub id:     xlib::Window,
	pub width:  u32,
	pub height: u32,

	pub display: Arc<Display>,
	pub screen:  c_int,
	pub root:    xlib::Window,
	pub cursor:  xlib::Cursor,
	pub im:      xlib::XIM,
	pub ic:      xlib::XIC,
}

unsafe impl Send for Window { }
unsafe impl Sync for Window { }

#[derive(Clone)]
pub enum Request {
	Sanitize,

	Start,
	Lock,
	Blank,
	Stop,
}

#[derive(Clone)]
pub enum Response {
	Activity,
	Keyboard(Keyboard),
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Keyboard {
	Char(char),
	Key(c_ulong, bool),
}

impl Locker {
	pub fn spawn(config: Config) -> error::Result<Locker> {
		unsafe {
			let     display = Arc::new(Display { id: xlib::XOpenDisplay(ptr::null()).as_mut().ok_or(error::Locker::NoDisplay)? });
			let mut windows = HashMap::new(): HashMap<xlib::Window, Arc<Window>>;
			let mut savers  = HashMap::new(): HashMap<xlib::Window, Saver>;

			xlib::XSetScreenSaver(display.id, 0, 0, 0, 0);

			for screen in 0 .. xlib::XScreenCount(display.id) {
				let root   = xlib::XRootWindow(display.id, screen);
				let width  = xlib::XDisplayWidth(display.id, screen) as c_uint;
				let height = xlib::XDisplayHeight(display.id, screen) as c_uint;
				let black  = xlib::XBlackPixelOfScreen(xlib::XDefaultScreenOfDisplay(display.id));

				let info = glx::glXChooseVisual(display.id, screen,
					[glx::GLX_RGBA, glx::GLX_DEPTH_SIZE, 24, glx::GLX_DOUBLEBUFFER, 0].as_ptr() as *mut _)
						.as_mut().ok_or(error::Locker::NoVisual)?;

				let colormap = xlib::XCreateColormap(display.id, root, (*info).visual, xlib::AllocNone);

				let id = {
					let mut attrs = mem::zeroed(): xlib::XSetWindowAttributes;
					let mut mask  = 0;

					mask |= xlib::CWColormap;
					attrs.colormap = colormap;

					mask |= xlib::CWBackingStore;
					attrs.backing_store = xlib::NotUseful;

					mask |= xlib::CWBackingPixel;
					attrs.backing_pixel = black;

					mask |= xlib::CWBorderPixel;
					attrs.border_pixel = black;

					mask |= xlib::CWOverrideRedirect;
					attrs.override_redirect = 1;

					mask |= xlib::CWEventMask;
					attrs.event_mask = xlib::KeyPressMask | xlib::KeyReleaseMask |
						xlib::ButtonPressMask | xlib::ButtonReleaseMask |
						xlib::PointerMotionMask | xlib::ExposureMask;

					xlib::XCreateWindow(display.id, root, 0, 0, width, height, 0, (*info).depth,
						xlib::InputOutput as c_uint, (*info).visual, mask, &mut attrs)
				};

				let cursor = {
					let bit    = xlib::XCreatePixmapFromBitmapData(display.id, id, b"\x00".as_ptr() as *const _ as *mut _, 1, 1, black, black, 1);
					let cursor = xlib::XCreatePixmapCursor(display.id, bit, bit, &mut mem::zeroed(), &mut mem::zeroed(), 0, 0);

					xlib::XFreePixmap(display.id, bit);
					xlib::XDefineCursor(display.id, id, cursor);

					cursor
				};

				let im = xlib::XOpenIM(display.id, ptr::null_mut(), ptr::null_mut(), ptr::null_mut())
					.as_mut().ok_or(error::Locker::NoIM)?;

				let ic = util::with("inputStyle", |input_style|
					util::with("clientWindow", |client_window|
						xlib::XCreateIC(im, input_style, xlib::XIMPreeditNothing | xlib::XIMStatusNothing,
							client_window, id, ptr::null_mut::<()>())))
								.as_mut().ok_or(error::Locker::NoIC)?;

				windows.insert(id, Arc::new(Window {
					id:     id,
					width:  width,
					height: height,

					display: display.clone(),
					screen:  screen,
					root:    root,
					cursor:  cursor,
					im:      im,
					ic:      ic,
				}));
			}

			let (sender, i_receiver) = channel();
			let (i_sender, receiver) = channel();

			// FIXME(meh): The whole `XPending` check and sleeping for 100ms to then
			//             `try_recv` on the channels is very fragile.
			//
			//             Find a better way to do it that plays well with Xlib's
			//             threading model (yeah, right).
			{
				let display = display.clone();

				thread::spawn(move || {
					let mut event = mem::zeroed(): xlib::XEvent;

					loop {
						// Check if there are any control messages.
						if let Ok(message) = receiver.try_recv() {
							match message {
								Request::Sanitize => {

								}

								Request::Start => {
									for window in windows.values() {
										let name = &config.savers()[rand::thread_rng().gen_range(0, config.savers().len())];

										if let Ok(saver) = Saver::spawn(name) {
											saver.config(config.saver(name)).unwrap();
											saver.target(CStr::from_ptr(xlib::XDisplayString(display.id)).to_str().unwrap(),
												window.screen, window.id).unwrap();

											savers.insert(window.id, saver);
										}
										else {
											// TODO(meh): blank the window

											xlib::XMapRaised(window.display.id, window.id);
										}
									}
								}

								Request::Lock => {

								}

								Request::Blank => {

								}

								Request::Stop => {
									for window in windows.values() {
										if let Some(saver) = savers.get_mut(&window.id) {
											saver.stop().unwrap();
										}
										else {
											xlib::XUnmapWindow(window.display.id, window.id);
										}
									}
								}
							}

							continue;
						}

						// Check if there are any messages from savers.
						{
							let mut stopped = Vec::new();

							for (&id, saver) in &savers {
								if let Ok(response) = saver.recv() {
									if response.is_none() {
										continue;
									}

									match response.unwrap() {
										saver::Response::Initialized => {
											saver.start().unwrap();
										}

										saver::Response::Started => {
											let window = windows.get(&id).unwrap();

											xlib::XMapRaised(window.display.id, window.id);
											xlib::XSync(window.display.id, xlib::False);
											xlib::XSetInputFocus(window.display.id, window.id, xlib::RevertToPointerRoot, xlib::CurrentTime);

											xlib::XGrabKeyboard(display.id, window.root, xlib::True,
												xlib::GrabModeAsync, xlib::GrabModeAsync, xlib::CurrentTime);

											xlib::XGrabPointer(display.id, window.root, xlib::False,
												(xlib::ButtonPressMask | xlib::ButtonReleaseMask | xlib::PointerMotionMask) as c_uint,
												xlib::GrabModeAsync, xlib::GrabModeAsync, 0, xlib::XBlackPixelOfScreen(xlib::XDefaultScreenOfDisplay(display.id)),
												xlib::CurrentTime);
										}

										saver::Response::Stopped => {
											stopped.push(id);
										}
									}
								}
								else {
									stopped.push(id);
								}
							}

							for id in &stopped {
								let window = windows.get(id).unwrap();

								xlib::XUnmapWindow(window.display.id, window.id);
								xlib::XUngrabKeyboard(window.display.id, xlib::CurrentTime);
								xlib::XUngrabPointer(window.display.id, xlib::CurrentTime);

								savers.remove(id);
							}
						}

						// Check if there are any pending events.
						if xlib::XPending(display.id) == 0 {
							thread::sleep(Duration::from_millis(100));
							continue;
						}

						xlib::XNextEvent(display.id, &mut event);
						let any = xlib::XAnyEvent::from(event);

						// Check if the event is from one of our windows.
						if let Some(window) = windows.values().find(|w| w.id == any.window) {
							match event.get_type() {
								xlib::KeyPress | xlib::KeyRelease => {
									let     key    = xlib::XKeyEvent::from(event);
									let     code   = key.keycode;
									let mut ic_sym = 0;

									let mut buffer = [0u8; 16];
									let     count  = xlib::Xutf8LookupString(window.ic, mem::transmute(&event),
										mem::transmute(buffer.as_mut_ptr()), buffer.len() as c_int,
										&mut ic_sym, ptr::null_mut());

									for ch in str::from_utf8(&buffer[..count as usize]).unwrap_or("").chars() {
										sender.send(Response::Keyboard(Keyboard::Char(ch))).unwrap();
									}

									let mut sym = xlib::XKeycodeToKeysym(window.display.id, code as xlib::KeyCode, 0);

									if keysym::XK_KP_Space as c_ulong <= sym && sym <= keysym::XK_KP_9 as c_ulong {
										sym = ic_sym;
									}

									sender.send(Response::Keyboard(Keyboard::Key(sym, event.get_type() == xlib::KeyPress))).unwrap();
								}

								other => {
									info!("event: {}", other);
								}
							}
						}
						else {

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

	pub fn sanitize(&self) {
		self.sender.send(Request::Sanitize).unwrap();
	}

	pub fn start(&self) {
		self.sender.send(Request::Start).unwrap();
	}

	pub fn lock(&self) {
		self.sender.send(Request::Lock).unwrap();
	}

	pub fn stop(&self) {
		self.sender.send(Request::Stop).unwrap();
	}

	pub fn blank(&self) {
		self.sender.send(Request::Blank).unwrap();
	}
}

impl AsRef<Receiver<Response>> for Locker {
	fn as_ref(&self) -> &Receiver<Response> {
		&self.receiver
	}
}

impl AsRef<Sender<Request>> for Locker {
	fn as_ref(&self) -> &Sender<Request> {
		&self.sender
	}
}

impl Drop for Display {
	fn drop(&mut self) {
		unsafe {
			xlib::XCloseDisplay(self.id);
		}
	}
}

impl Drop for Window {
	fn drop(&mut self) {
		unsafe {
			xlib::XDestroyWindow(self.display.id, self.id);
		}
	}
}
