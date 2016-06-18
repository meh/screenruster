use std::ptr;
use std::mem;
use std::str;
use std::thread;
use std::time::Duration;
use std::sync::mpsc::{Receiver, Sender, SendError, channel};
use std::sync::Arc;

use x11::{xlib, glx, keysym};
use libc::{c_int, c_uint, c_ulong};
use image;

use error;
use config;
use util;

pub struct Window {
	receiver: Receiver<Response>,
	sender:   Sender<Request>,
	instance: Arc<Instance>,
}

pub struct Instance {
	pub id:     xlib::Window,
	pub width:  u32,
	pub height: u32,

	pub display: *mut xlib::Display,
	pub root:    xlib::Window,
	pub context: glx::GLXContext,
	pub cursor:  xlib::Cursor,
	pub im:      xlib::XIM,
	pub ic:      xlib::XIC,
}

unsafe impl Send for Instance { }
unsafe impl Sync for Instance { }

#[derive(Clone)]
pub enum Request {
	Show,
	Hide,
	Sanitize,
	Screenshot,
}

#[derive(Clone)]
pub enum Response {
	Activity,
	Keyboard(Keyboard),
	Screenshot(image::DynamicImage),
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Keyboard {
	Char(char),
	Key(c_ulong, bool),
}

impl Window {
	pub fn spawn(config: config::Window) -> error::Result<Window> {
		unsafe {
			xlib::XInitThreads();

			let display = xlib::XOpenDisplay(ptr::null())
				.as_mut().ok_or(error::Window::NoDisplay)?;

			let screen = xlib::XDefaultScreen(display);
			let root   = xlib::XRootWindow(display, screen);
			let width  = xlib::XDisplayWidth(display, screen) as c_uint;
			let height = xlib::XDisplayHeight(display, screen) as c_uint;
			let black  = xlib::XBlackPixelOfScreen(xlib::XDefaultScreenOfDisplay(display));

			let info = glx::glXChooseVisual(display, screen,
				[glx::GLX_RGBA, glx::GLX_DEPTH_SIZE, 24, glx::GLX_DOUBLEBUFFER, 0].as_ptr() as *mut _)
					.as_mut().ok_or(error::Window::NoVisual)?;

			let colormap = xlib::XCreateColormap(display, root, (*info).visual, xlib::AllocNone);

			let window = {
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

				xlib::XCreateWindow(display, root, 0, 0, width, height, 0, (*info).depth,
					xlib::InputOutput as c_uint, (*info).visual, mask, &mut attrs)
			};

			let cursor = {
				let bit    = xlib::XCreatePixmapFromBitmapData(display, window, b"\x00".as_ptr() as *const _ as *mut _, 1, 1, black, black, 1);
				let cursor = xlib::XCreatePixmapCursor(display, bit, bit, &mut mem::zeroed(), &mut mem::zeroed(), 0, 0);

				xlib::XFreePixmap(display, bit);
				xlib::XDefineCursor(display, window, cursor);

				cursor
			};

			let context = glx::glXCreateContext(display, info, ptr::null_mut(), 1)
				.as_mut().ok_or(error::Window::NoContext)?;

			let im = xlib::XOpenIM(display, ptr::null_mut(), ptr::null_mut(), ptr::null_mut())
				.as_mut().ok_or(error::Window::NoIM)?;

			let ic = util::with("inputStyle", |input_style|
				util::with("clientWindow", |client_window|
					xlib::XCreateIC(im, input_style, xlib::XIMPreeditNothing | xlib::XIMStatusNothing,
						client_window, window, ptr::null_mut::<()>())))
							.as_mut().ok_or(error::Window::NoIC)?;

			xlib::XSetScreenSaver(display, 0, 0, 0, 0);

			let instance = Arc::new(Instance {
				id:     window,
				width:  width,
				height: height,

				display: display,
				root:    root,
				context: context,
				cursor:  cursor,
				im:      im,
				ic:      ic,
			});

			let (sender, i_receiver) = channel();
			let (i_sender, receiver) = channel();

			{
				let instance = instance.clone();

				thread::spawn(move || unsafe {
					let mut event = mem::zeroed(): xlib::XEvent;

					loop {
						if let Ok(message) = receiver.try_recv() {
							match message {
								Request::Sanitize => {

								}

								Request::Show => {
									xlib::XMapRaised(instance.display, instance.id);
									xlib::XSetInputFocus(instance.display, instance.id,
										xlib::RevertToParent, xlib::CurrentTime);
								}

								Request::Hide => {
									xlib::XUnmapWindow(instance.display, instance.id);
								}

								Request::Screenshot => {
									let ximage = xlib::XGetImage(instance.display, instance.root,
										0, 0, instance.width, instance.height,
										xlib::XAllPlanes(), xlib::ZPixmap)
											.as_mut().unwrap();

									let r = (*ximage).red_mask;
									let g = (*ximage).green_mask;
									let b = (*ximage).blue_mask;

									let mut image = image::DynamicImage::new_rgb8(instance.width, instance.height);

									for (x, y, px) in image.as_mut_rgb8().unwrap().enumerate_pixels_mut() {
										let pixel = xlib::XGetPixel(ximage, x as c_int, y as c_int);

										px[0] = ((pixel & r) >> 16) as u8;
										px[1] = ((pixel & g) >> 8)  as u8;
										px[2] = ((pixel & b) >> 0)  as u8;
									}

									xlib::XDestroyImage(ximage);

									sender.send(Response::Screenshot(image)).unwrap()
								}
							}

							continue;
						}

						if xlib::XPending(instance.display) == 0 {
							thread::sleep(Duration::from_millis(100));

							continue;
						}

						xlib::XNextEvent(instance.display, &mut event);

						match event.get_type() {
							xlib::KeyPress | xlib::KeyRelease => {
								let     press  = event.get_type() == xlib::KeyPress;
								let mut key    = xlib::XKeyEvent::from(event);
								let     code   = key.keycode;
								let mut ic_sym = 0;

								let mut buffer = [0u8; 16];
								let     count  = xlib::Xutf8LookupString(instance.ic, mem::transmute(&event),
									mem::transmute(buffer.as_mut_ptr()), buffer.len() as c_int,
									&mut ic_sym, ptr::null_mut());

								for ch in str::from_utf8(&buffer[..count as usize]).unwrap_or("").chars() {
									sender.send(Response::Keyboard(Keyboard::Char(ch))).unwrap();
								}

								let mut sym = xlib::XKeycodeToKeysym(instance.display, code as xlib::KeyCode, 0);

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
				});
			}

			Ok(Window {
				receiver: i_receiver,
				sender:   i_sender,
				instance: instance,
			})
		}
	}

	pub fn instance(&self) -> Arc<Instance> {
		self.instance.clone()
	}

	pub fn sanitize(&self) {
		self.sender.send(Request::Sanitize).unwrap();
	}

	pub fn show(&self) {
		self.sender.send(Request::Show).unwrap();
	}

	pub fn hide(&self) {
		self.sender.send(Request::Hide).unwrap();
	}

	pub fn screenshot(&self) {
		self.sender.send(Request::Screenshot).unwrap();
	}
}

impl AsRef<Receiver<Response>> for Window {
	fn as_ref(&self) -> &Receiver<Response> {
		&self.receiver
	}
}

impl AsRef<Sender<Request>> for Window {
	fn as_ref(&self) -> &Sender<Request> {
		&self.sender
	}
}

impl Drop for Instance {
	fn drop(&mut self) {
		unsafe {
			glx::glXDestroyContext(self.display, self.context);
			xlib::XDestroyWindow(self.display, self.id);
			xlib::XCloseDisplay(self.display);
		}
	}
}
