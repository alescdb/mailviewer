pub mod application;
pub mod config;
pub mod html;
pub mod mailparser;
pub mod window;

pub use mailparser::MailParser;
pub use application::MailViewerApplication;
pub use window::MailViewerWindow;
pub use html::Html;
