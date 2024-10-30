use crate::{
  config::VERSION,
  mailparser::{Attachment, MailParser},
};
use std::{cell::RefCell, path::Path};

pub struct MailService {
  parser: Option<MailParser>,
  fullpath: Option<String>,
  show_file_name: RefCell<bool>,
  signal_title_changed: RefCell<Option<Box<dyn Fn(&Self, &str) + 'static>>>,
}

impl MailService {
  pub fn new() -> Self {
    Self {
      parser: None,
      fullpath: None,
      show_file_name: RefCell::new(true),
      signal_title_changed: RefCell::new(None),
    }
  }

  pub fn open_mail(&mut self, fullpath: &str) -> Result<bool, Box<dyn std::error::Error>> {
    if Path::new(fullpath).exists() == false {
      return Ok(false);
    }
    self.fullpath = Some(fullpath.to_string());
    self.parser = Some(MailParser::new(fullpath));
    self.parser.as_mut().unwrap().parse()?;
    self.update_title();
    Ok(true)
  }

  pub fn set_show_file_name(&self, show_file_name: bool) {
    self.show_file_name.replace(show_file_name);
    self.update_title();
  }

  pub fn get_fullpath(&self) -> Option<String> {
    self.fullpath.clone()
  }

  pub fn get_parser(&self) -> Option<MailParser> {
    self.parser.clone()
  }

  pub fn connect_title_changed<F: Fn(&Self, &str) + 'static>(&self, f: F) {
    self.signal_title_changed.borrow_mut().replace(Box::new(f));
    self.update_title();
  }

  fn update_title(&self) {
    if let Some(callback) = self.signal_title_changed.borrow_mut().take() {
      if let Some(fullpath) = &self.fullpath {
        let title = self.get_title(fullpath);
        callback(self, &title);
      }
    }
  }

  pub fn get_attachments(&self) -> &Vec<Attachment> {
    self.parser.as_ref().unwrap().attachments.as_ref()
  }

  fn get_title(&self, fullpath: &str) -> String {
    if *self.show_file_name.borrow() {
      if let Some(filename) = Path::new(fullpath).file_name() {
        return filename.to_string_lossy().to_string();
      }
    }
    format!("Mail Viewer v{}", VERSION)
  }
}

impl std::fmt::Debug for MailService {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("MailService")
      .field("parser", &self.parser)
      .field("fullpath", &self.fullpath)
      .field("show_file_name", &self.show_file_name)
      .finish()
  }
}
