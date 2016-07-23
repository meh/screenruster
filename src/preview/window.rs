use std::sync::Arc;
use std::ops::Deref;

use xcb;

use error;
use platform::{self, Display};

pub struct Window {
	display: Arc<Display>,
	window:  platform::Window,
}

impl Window {
	pub fn create(display: Arc<Display>) -> error::Result<Window> {
		let screen = display.get_setup().roots().nth(display.screen() as usize).unwrap();
		let window = platform::Window::create(display.clone(), display.screen(),
			(screen.width_in_pixels() as f32 / 1.2) as u32,
			(screen.height_in_pixels() as f32 / 1.2) as u32)?;

		xcb::change_property(&display, xcb::PROP_MODE_REPLACE as u8, window.id(),
			xcb::ATOM_WM_NAME, xcb::ATOM_STRING, 8, b"ScreenRuster");

		// TODO: set size hints when that's added to xcb

		display.flush();

		Ok(Window {
			display: display.clone(),
			window:  window,
		})
	}

	pub fn show(&self) {
		xcb::map_window(&self.display, self.id());

		self.flush();
	}
}

impl Deref for Window {
	type Target = platform::Window;

	fn deref(&self) -> &Self::Target {
		&self.window
	}
}
