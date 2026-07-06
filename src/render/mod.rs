//! Terminal 3D rendering. Knows nothing about physics beyond reading body state.

pub mod camera;
pub mod cell;
pub mod framebuffer;
pub mod scale;
pub mod scene;
pub mod terminal;

pub use camera::Camera;
pub use framebuffer::FrameBuffer;
