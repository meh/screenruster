use std::rc::Rc;

use glium::{self, Surface};
use toml;

use error;

pub struct Saver {
	config: toml::Table,
	state:  super::State,
	gl:     Option<Graphics>
}

unsafe impl Send for Saver { }

pub struct Graphics {
	vertex:  glium::VertexBuffer<Vertex>,
	index:   glium::IndexBuffer<u16>,
	program: glium::Program,
}

#[derive(Copy, Clone)]
struct Vertex {
	position: [f32; 2],
	texture:  [f32; 2],
}

implement_vertex!(Vertex, position, texture);

impl Saver {
	pub fn new(config: toml::Table) -> error::Result<Saver> {
		Ok(Saver {
			config: config,
			state:  Default::default(),
			gl:     None,
		})
	}
}

impl super::Saver for Saver {
	fn initialize(&mut self, context: Rc<glium::backend::Context>) {
		let vertex = glium::VertexBuffer::new(&context, &[
			Vertex { position: [-1.0, -1.0], texture: [0.0, 0.0] },
			Vertex { position: [-1.0,  1.0], texture: [0.0, 1.0] },
			Vertex { position: [ 1.0,  1.0], texture: [1.0, 1.0] },
			Vertex { position: [ 1.0, -1.0], texture: [1.0, 0.0] },
		]).unwrap();

		let index = glium::IndexBuffer::new(&context, glium::index::PrimitiveType::TriangleStrip,
			&[1 as u16, 2, 0, 3]).unwrap();

		let program = program!(&context,
			110 => {
				vertex: "
					#version 110

					uniform mat4 matrix;

					attribute vec2 position;
					attribute vec2 texture;

					varying vec2 v_texture;

					void main() {
						gl_Position = matrix * vec4(position, 0.0, 1.0);
						v_texture = texture;
					}
				",

				fragment: "
					#version 110
					uniform sampler2D screen;
					varying vec2 v_texture;

					void main() {
						gl_FragColor = vec4(texture2D(screen, v_texture).rgb, 0.4);
					}
				",
			},
		).unwrap();

		self.gl = Some(Graphics {
			vertex:  vertex,
			index:   index,
			program: program,
		});
	}

	fn begin(&mut self) {
		self.state = super::State::Running;
	}

	fn end(&mut self) {
		self.state = super::State::None;
	}

	fn state(&self) -> super::State {
		self.state
	}

	fn render(&self, target: &mut glium::Frame, screen: &glium::texture::Texture2d) {
		let gl = self.gl.as_ref().unwrap();

		let uniforms = uniform! {
			matrix: [
				[1.0, 0.0, 0.0, 0.0],
				[0.0, 1.0, 0.0, 0.0],
				[0.0, 0.0, 1.0, 0.0],
				[0.0, 0.0, 0.0, 1.0f32]
			],

			screen: screen,
		};

		target.draw(&gl.vertex, &gl.index, &gl.program, &uniforms, &glium::DrawParameters {
			blend: glium::Blend {
				color: glium::BlendingFunction::Addition {
					source:      glium::LinearBlendingFactor::SourceAlpha,
					destination: glium::LinearBlendingFactor::OneMinusSourceAlpha
				},

				alpha: glium::BlendingFunction::Addition {
					source:      glium::LinearBlendingFactor::SourceAlpha,
					destination: glium::LinearBlendingFactor::OneMinusSourceAlpha
				},

				.. Default::default()
			},

			.. Default::default()
		}).unwrap();
	}
}
