use std::ptr;
use std::mem;
use std::thread;
use std::ops::Deref;
use std::sync::mpsc::{Receiver, Sender, SendError, channel};
use std::sync::Arc;

use x11::{xlib, glx};
use libc::{c_uint};

use error;
use config;

pub struct Window {
	receiver: Receiver<Event>,
	sender:   Sender<Event>,
	instance: Arc<Instance>,
}

pub struct Instance {
	pub display: *mut xlib::Display,
	pub id:      xlib::Window,
	pub context: glx::GLXContext,

	pub width:  u32,
	pub height: u32,
}

unsafe impl Send for Instance { }
unsafe impl Sync for Instance { }

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Event {
	Show,
	Hide,
	Activity,
}

impl Window {
	pub fn spawn(config: config::Window) -> error::Result<Window> {
		unsafe {
			let display = xlib::XOpenDisplay(ptr::null())
				.as_mut().ok_or(error::Window::NoDisplay)?;

			let screen = xlib::XDefaultScreen(display);
			let root   = xlib::XRootWindow(display, screen);

			let info = glx::glXChooseVisual(display, screen,
				[glx::GLX_RGBA, glx::GLX_DEPTH_SIZE, 24, glx::GLX_DOUBLEBUFFER, glx::GLX_LEVEL, 3, 0].as_ptr() as *mut _)
					.as_mut().ok_or(error::Window::NoVisual)?;

			let colormap = xlib::XCreateColormap(display, root, (*info).visual, xlib::AllocNone);

			let mut attrs = mem::zeroed(): xlib::XSetWindowAttributes;
			attrs.colormap = colormap;
			attrs.event_mask = xlib::KeyPressMask | xlib::KeyReleaseMask |
				xlib::ButtonPressMask | xlib::ButtonReleaseMask |
				xlib::PointerMotionMask | xlib::ExposureMask;

			let screen = xlib::XDefaultScreenOfDisplay(display);
			let width  = (*screen).width as c_uint;
			let height = (*screen).height as c_uint;

			let window = xlib::XCreateWindow(display, root, 0, 0, width, height, 0, (*info).depth,
				xlib::InputOutput as c_uint, (*info).visual, xlib::CWColormap | xlib::CWEventMask, &mut attrs);

			let context = glx::glXCreateContext(display, info, ptr::null_mut(), 1)
				.as_mut().ok_or(error::Window::NoContext)?;

			let instance = Arc::new(Instance {
				display: display,
				id:      window,
				context: context,

				width:  width,
				height: height,
			});

			let (sender, i_receiver) = channel();
			let (i_sender, receiver) = channel();

			// event reception thread
			{
				let instance = instance.clone();

				thread::spawn(move || unsafe {
					let mut event = mem::zeroed(): xlib::XEvent;

					loop {
						xlib::XNextEvent(instance.display, &mut event);
						sender.send(Event::Activity);
					}
				});
			}

			// window change thread
			{
				let instance = instance.clone();

				thread::spawn(move || unsafe {
					loop {
						match receiver.recv() {
							Ok(Event::Show) => {
								xlib::XMapRaised(instance.display, instance.id);
							}

							Ok(Event::Hide) => {
								xlib::XUnmapWindow(instance.display, instance.id);
							}

							Ok(_)  => (),
							Err(_) => break,
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

	pub fn send(&self, event: Event) -> Result<(), SendError<Event>> {
		self.sender.send(event)
	}
}

impl Deref for Window {
	type Target = Receiver<Event>;

	fn deref(&self) -> &Self::Target {
		&self.receiver
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
