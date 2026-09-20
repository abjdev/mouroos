pub mod apps;
pub mod color;
pub mod desktop;
pub mod font;
pub mod framebuffer;
pub mod icons;
pub mod theme;
pub mod welcome;
pub mod window;

pub use color::Color;
pub use desktop::{Desktop, run_desktop};
pub use framebuffer::Framebuffer;
pub use theme::{Theme, ThemeKind};
pub use welcome::show_welcome_screen;
pub use window::{Application, Window};
