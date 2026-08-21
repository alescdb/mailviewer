use crate::message::attachment::Attachment;

#[derive(Debug)]
pub(crate) struct ViewerContent {
  pub html: Option<String>,
  pub text: Option<String>,
  pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RenderOptions {
  pub force_css: bool,
  pub allow_remote: bool,
  pub dark: bool,
}
