//! Shared scene-rendering state for interactive and headless output paths.

use super::camera::Camera;
use super::framebuffer::FrameBuffer;
use super::scale::ScaleMode;
use super::scene::{self, Fill, Representation};
use super::starfield::{self, Star};
use crate::scenario::Loaded;
use crate::sim::World;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderOptions {
    pub scale: ScaleMode,
    pub representation: Representation,
    pub fill: Fill,
    pub chrome: bool,
}

impl RenderOptions {
    pub fn from_loaded(loaded: &Loaded) -> Self {
        Self {
            scale: ScaleMode::from_name(&loaded.scale).unwrap_or(ScaleMode::Compressed),
            representation: Representation::from_name(&loaded.representation)
                .unwrap_or(Representation::Heliocentric),
            fill: Fill::from_name(&loaded.fill).unwrap_or(Fill::Blocks),
            chrome: loaded.show_labels,
        }
    }
}

pub struct RenderSession {
    pub camera: Camera,
    pub options: RenderOptions,
    framebuffer: FrameBuffer,
    stars: Vec<Star>,
}

impl RenderSession {
    pub fn new(
        width: u16,
        height: u16,
        camera: Camera,
        options: RenderOptions,
        star_count: usize,
    ) -> Self {
        Self {
            camera,
            options,
            framebuffer: FrameBuffer::new(width, height),
            stars: starfield::generate(star_count),
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.framebuffer.resize(width, height);
    }

    pub fn framebuffer(&self) -> &FrameBuffer {
        &self.framebuffer
    }

    pub fn framebuffer_mut(&mut self) -> &mut FrameBuffer {
        &mut self.framebuffer
    }

    pub fn render_scene(&mut self, world: &World, selected: usize) {
        self.framebuffer.clear();
        scene::render(
            &mut self.framebuffer,
            &self.camera,
            world,
            selected,
            &self.stars,
            self.options,
        );
        self.framebuffer.composite_pixels();
        self.framebuffer.composite_braille();
    }

    pub fn render_text(&mut self, world: &World, selected: usize) -> String {
        self.render_scene(world, selected);
        self.framebuffer.to_text()
    }
}
