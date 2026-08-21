use crate::message::attachment::Attachment;

#[derive(Debug)]
pub(crate) struct PrintableContent {
  pub html: Option<String>,
  pub text: Option<String>,
  pub from: String,
  pub to: String,
  pub date: String,
  pub subject: String,
  pub attachments: Vec<Attachment>,
}

pub trait Printable {
  fn print(
    &self,
    parent: &gtk4::Window,
    content: &PrintableContent,
    options: crate::viewer::content::RenderOptions,
  );
}
