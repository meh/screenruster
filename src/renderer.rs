use std::thread;
use std::time::Instant;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::Arc;
use std::ptr;
use std::ffi::CString;
use std::os::raw::c_void;

use x11::glx;
use api::gl::{self, Surface};
use api::image::{self, GenericImage};
use api::{self, Saver};

use error;
use window;
use util::DurationExt;

pub struct Renderer {
	receiver: Receiver<Response>,
	sender:   Sender<Request>,
}

pub struct Backend {
	pub window: Arc<window::Instance>,
}

pub enum Request {
	Initialize(Box<Saver>),
	Start(image::DynamicImage),
	Dialog(bool),
	Stop,
}

pub enum Response {
	Initialized,
	Started,
	Stopped,
}

impl Renderer {
	// TODO: catch panics when dealing with the `saver` and use sane defaults in
	// case of panic.
	pub fn spawn(window: Arc<window::Instance>) -> error::Result<Renderer> {
		let (sender, i_receiver) = channel();
		let (i_sender, receiver) = channel();

		thread::spawn(move || {
			let context = unsafe {
				gl::backend::Context::new::<_, ()>(Backend { window: window },
					false, Default::default()).unwrap()
			};

			let mut state  = api::State::default();
			let mut saver  = None: Option<Box<Saver>>;
			let mut screen = None: Option<gl::texture::Texture2d>;

			loop {
				if let api::State::None = state {
					match receiver.recv().unwrap() {
						Request::Initialize(mut sv) => {
							sv.initialize(context.clone());
							saver = Some(sv);

							sender.send(Response::Initialized).unwrap();
						}

						Request::Start(sc) => {
							state  = api::State::Begin;
							screen = Some({
								let size  = sc.dimensions();
								let image = gl::texture::RawImage2d::from_raw_rgba_reversed(
									sc.to_rgba().into_raw(), size);

								gl::texture::Texture2d::new(&context, image).unwrap()
							});
						}

						_ => ()
					}

					continue;
				}

				let mut saver  = saver.take().unwrap();
				let     screen = screen.take().unwrap();
				let     step   = (saver.step() * 1_000_000.0).round() as u64;

				sender.send(Response::Started).unwrap();
				saver.begin();

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

						if state == api::State::None {
							state = api::State::None;
							break 'render;
						}

						lag -= step;
					}

					// Check if we received any requests.
					if let Ok(event) = receiver.try_recv() {
						match event {
							Request::Stop => {
								saver.end();
							}

							Request::Dialog(active) => {
								saver.dialog(active);
							}

							_ => ()
						}
					}

					let mut target = gl::Frame::new(context.clone(), context.get_framebuffer_dimensions());
					target.clear_all((0.0, 0.0, 0.0, 1.0), 1.0, 0);
					saver.render(&mut target, &screen);
					target.finish().unwrap();
				}

				sender.send(Response::Stopped).unwrap();
			}
		});

		Ok(Renderer {
			receiver: i_receiver,
			sender:   i_sender,
		})
	}

	pub fn initialize(&self, saver: Box<Saver>) {
		self.sender.send(Request::Initialize(saver)).unwrap();
	}

	pub fn start(&self, screen: image::DynamicImage) {
		self.sender.send(Request::Start(screen)).unwrap();
	}

	pub fn stop(&self) {
		self.sender.send(Request::Stop).unwrap();
	}
}

impl AsRef<Receiver<Response>> for Renderer {
	fn as_ref(&self) -> &Receiver<Response> {
		&self.receiver
	}
}

impl AsRef<Sender<Request>> for Renderer {
	fn as_ref(&self) -> &Sender<Request> {
		&self.sender
	}
}

unsafe impl gl::backend::Backend for Backend {
	fn swap_buffers(&self) -> Result<(), gl::SwapBuffersError> {
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
