use glium;

pub struct Saver {
	state: super::State,
}

impl super::Saver for Saver {
	fn lock(&mut self) {
		self.state = super::State::Locked;
	}

	fn unlock(&mut self) {
		self.state = super::State::Unlocked;
	}

	fn state(&self) -> super::State {
		self.state
	}

	fn update(&mut self) {

	}

	fn render(&self, surface: glium::Frame) {

	}
}
