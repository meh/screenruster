use std::thread;
use std::time::{Instant, Duration};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::Arc;
use std::ptr;
use std::ffi::CString;
use std::os::raw::c_void;

use glium::{self, Surface};
use x11::glx;
use image::{self, GenericImage};

use error;
use window;
use saver::{self, Saver};
use util::DurationExt;

pub struct Renderer {
	receiver: Receiver<Event>,
	sender:   Sender<Event>,
}

pub struct Backend {
	pub window: Arc<window::Instance>,
}

pub enum Event {
	State(saver::State),

	Start {
		saver:  Box<Saver>,
		screen: image::DynamicImage,
	},

	Stop,
}

impl Renderer {
	// TODO: catch panics when dealing with the `saver` and use sane defaults in
	// case of panic.
	pub fn spawn(window: Arc<window::Instance>) -> error::Result<Renderer> {
		let (sender, i_receiver) = channel();
		let (i_sender, receiver) = channel();

		thread::spawn(move || {
			let context = unsafe {
				glium::backend::Context::new::<_, ()>(Backend { window: window }, false, Default::default()).unwrap()
			};

			let mut state  = saver::State::Idle;
			let mut saver  = None: Option<Box<Saver>>;
			let mut screen = None: Option<glium::texture::Texture2d>;

			loop {
				if let saver::State::Idle = state {
					match receiver.recv().unwrap() {
						Event::Start { saver: sv, screen: sc } => {
							state  = saver::State::Starting;
							saver  = Some(sv);
							screen = Some({
								let size  = sc.dimensions();
								let image = glium::texture::RawImage2d::from_raw_rgba_reversed(sc.to_rgba().into_raw(), size);

								glium::texture::Texture2d::new(&context, image).unwrap()
							});
						}

						_ => ()
					}

					continue;
				}

				let mut saver  = saver.take().unwrap();
				let     screen = screen.take().unwrap();
				let     step   = (saver.step() * 1_000_000.0).round() as u64;

				saver.initialize(context.clone());
				sender.send(Event::State(saver::State::Starting)).unwrap();
				saver.start();

				if state != saver.state() {
					sender.send(Event::State(saver.state())).unwrap();
					state = saver.state();
				}

				let mut lag      = 0;
				let mut previous = Instant::now();

				'render: loop {
					let now     = Instant::now();
					let elapsed = now.duration_since(previous);

					previous  = now;
					lag      += elapsed.as_nanosecs();

					// Update the state.
					while lag >= step {
						saver.update();

						if state != saver.state() {
							sender.send(Event::State(saver.state())).unwrap();
							state = saver.state();
						}

						if state == saver::State::Stopped {
							state = saver::State::Idle;
							break 'render;
						}

						lag -= step;
					}

					// Check if we received any events.
					if let Ok(event) = receiver.try_recv() {
						match event {
							Event::Stop => {
								saver.stop();
							}

							_ => ()
						}
					}

					let mut target = glium::Frame::new(context.clone(), context.get_framebuffer_dimensions());
					target.clear_all((0.0, 0.0, 0.0, 1.0), 1.0, 0);
					saver.render(&mut target, &screen);
					target.finish().unwrap();
				}
			}
		});

		Ok(Renderer {
			receiver: i_receiver,
			sender:   i_sender,
		})
	}

	pub fn start(&self, saver: Box<Saver>, screen: image::DynamicImage) {
		self.sender.send(Event::Start { saver: saver, screen: screen }).unwrap();
	}

	pub fn stop(&self) {
		self.sender.send(Event::Stop).unwrap();
	}
}

impl AsRef<Receiver<Event>> for Renderer {
	fn as_ref(&self) -> &Receiver<Event> {
		&self.receiver
	}
}

impl AsRef<Sender<Event>> for Renderer {
	fn as_ref(&self) -> &Sender<Event> {
		&self.sender
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
