use std::rc::Rc;

use glium;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum State {
	None,
	Begin,
	Running,
	End,
}

impl Default for State {
	fn default() -> State {
		State::None
	}
}

pub trait Saver: Send {
	/// The update step for the render loop.
	fn step(&self) -> f64 { 0.015 }

	/// Initialize any GL related stuff.
	fn initialize(&mut self, context: Rc<glium::backend::Context>) { }

	/// The dialog is now `active` or not.
	fn dialog(&mut self, active: bool) { }

	/// The saver has been started, useful to implement a fade in or animation to
	/// only show at the beginning.
	fn begin(&mut self);

	/// The saver has been stopped, useful to implement a fade out or animation
	/// to show at the end.
	fn end(&mut self);

	/// Return the current saver state.
	fn state(&self) -> State;

	/// Called each `step` milliseconds.
	fn update(&mut self) { }

	/// Render the saver.
	fn render(&self, target: &mut glium::Frame, screen: &glium::texture::Texture2d) { }
}

pub mod laughing_man;
