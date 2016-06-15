use glium;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum State {
	Idle,
	Locking,
	Locked,
	Unlocking,
	Unlocked,
}

impl Default for State {
	fn default() -> State {
		State::Idle
	}
}

pub trait Saver: Send {
	fn step(&self) -> u64 {
		15
	}

	fn lock(&mut self);
	fn unlock(&mut self);

	fn state(&self) -> State;
	fn update(&mut self);
	fn render(&self, target: glium::Frame);
}

pub mod laughing_man;
