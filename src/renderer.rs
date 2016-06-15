use std::thread;
use std::time::{Instant, Duration};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::Arc;
use std::ops::Deref;
use std::ptr;
use std::ffi::CString;
use std::os::raw::c_void;

use glium;
use x11::glx;

use error;
use window;
use saver::{self, Saver};

pub struct Renderer {
	receiver: Receiver<Event>,
	sender:   Sender<Event>,
}

pub struct Backend {
	pub window: Arc<window::Instance>,
}

pub enum Event {
	State(saver::State),
	Run(Box<Saver>),
}

impl Renderer {
	pub fn spawn(window: Arc<window::Instance>) -> error::Result<Renderer> {
		let context = unsafe {
			glium::backend::Context::new(Backend { window: window }, false, Default::default())?
		};

		let (sender, i_receiver) = channel();
		let (i_sender, receiver) = channel();

		thread::spawn(move || {
			let mut state    = saver::State::Idle;

//			loop {
//
//			}
//
//			let mut lag      = 0;
//			let mut previous = Instant::now();
//			let mut state    = Default::default();
//
//			loop {
//				let current = Instant::now();
//				let elapsed = current.duration_since(previous);
//
//				previous  = current;
//				lag      += elapsed.as_msecs();
//
//				while lag >= step {
//					sender.send(Event::State(saver.update());
//					lag -= step;
//				}
//
//				let mut target = glium::Frame::new(self.context.clone(), self.context.get_framebuffer_dimensions());
//				target.clear_all((1.0, 1.0, 1.0, 1.0), 1.0, 0);
//				saver.render(target);
//
//				match target.finish() {
//					Err(ContextLost) => {
//
//					}
//				}
//			}
		});

		Ok(Renderer {
			receiver: i_receiver,
			sender:   i_sender,
		})
	}

	pub fn run(&self, saver: Box<Saver>) {
		self.sender.send(Event::Run(saver));
	}
}

impl Deref for Renderer {
	type Target = Receiver<Event>;

	fn deref(&self) -> &Self::Target {
		&self.receiver
	}
}

unsafe impl glium::backend::Backend for Backend {
	fn swap_buffers(&self) -> Result<(), glium::SwapBuffersError> {
		unsafe {
			glx::glXSwapBuffers(self.window.display, self.window.id);
		}

		Ok(())
	}

	unsafe fn get_proc_address(&self, symbol: &str) -> *const c_void {
		let addr = CString::new(symbol.as_bytes()).unwrap();

		if let Some(func) = glx::glXGetProcAddress(addr.as_ptr() as *const _) {
			func as *const _
		}
		else {
			ptr::null()
		}
	}

	fn get_framebuffer_dimensions(&self) -> (u32, u32) {
		(self.window.width, self.window.height)
	}

	fn is_current(&self) -> bool {
		unimplemented!()
	}

	unsafe fn make_current(&self) {
		glx::glXMakeCurrent(self.window.display, self.window.id, self.window.context);
	}
}
