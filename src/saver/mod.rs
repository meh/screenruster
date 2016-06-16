use std::rc::Rc;

use glium;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum State {
	Idle,
	Starting,
	Started,
	Running,
	Stopping,
	Stopped,
}

impl Default for State {
	fn default() -> State {
		State::Idle
	}
}

pub trait Saver: Send {
	fn step(&self) -> f64 {
		0.015
	}

	fn initialize(&mut self, context: Rc<glium::backend::Context>);
	fn start(&mut self);
	fn stop(&mut self);

	fn state(&self) -> State;
	fn update(&mut self);
	fn render(&self, target: &mut glium::Frame, screen: &glium::texture::Texture2d);
}

pub mod laughing_man;
