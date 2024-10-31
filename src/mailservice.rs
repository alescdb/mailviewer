use crate::{config::VERSION, mailparser::{Attachment, MailParser}};
use std::{cell::RefCell, path::Path};

pub struct MailService {
  parser: RefCell<Option<MailParser>>,
  fullpath: RefCell<Option<String>>,
  show_file_name: RefCell<bool>,
  signal_title_changed: RefCell<Option<Box<dyn Fn(&Self, &str) + 'static>>>,
}

impl MailService {
  pub fn new() -> Self {
    Self {
      parser: RefCell::new(None),
      fullpath: RefCell::new(None),
      show_file_name: RefCell::new(true),
      signal_title_changed: RefCell::new(None),

      
    }
  }

  pub fn open_mail(&self, fullpath: &str) -> Result<(), Box<dyn std::error::Error>> {
    if Path::new(fullpath).exists() == false {
      return Err(format!("File not found : {}", fullpath).into());
    }
    self.fullpath.borrow_mut().replace(fullpath.to_string());
    let mut parser = MailParser::new(fullpath);
    parser.parse()?;
    self.parser.borrow_mut().replace(parser);
    self.update_title();
    Ok(())
  }

  pub fn get_from(&self) -> String {
    if let Some(parser) = self.parser.borrow().as_ref() {
      return parser.from.clone();
    }
    String::new()
  }

  pub fn get_to(&self) -> String {
    if let Some(parser) = self.parser.borrow().as_ref() {
      return parser.to.clone();
    }
    String::new()
  }

  pub fn get_subject(&self) -> String {
    if let Some(parser) = self.parser.borrow().as_ref() {
      return parser.subject.clone();
    }
    String::new()
  }

  pub fn get_date(&self) -> String {
    if let Some(parser) = self.parser.borrow().as_ref() {
      return parser.date.clone();
    }
    String::new()
  }

  pub fn get_text(&self) -> Option<String> {
    if let Some(parser) = self.parser.borrow().as_ref() {
      if let Some(text) = parser.body_text.clone() { 
        let proper = text.replace("\r\n", "\n");
        return Some(proper);          
      }
    }
    None
  }

  pub fn get_html(&self) -> Option<String> {
    if let Some(parser) = self.parser.borrow().as_ref() {
      if let Some(html) = parser.body_html.clone() {
        return Some(html.clone());
      }
    }
    None
  }

  pub fn get_attachments(&self) -> Vec<Attachment> {
    if let Some(parser) = self.parser.borrow().as_ref() {
      return parser.attachments.clone();
    }
    vec![]
  }

  pub fn set_show_file_name(&self, show_file_name: bool) {
    log::debug!("set_show_file_name({})", show_file_name);
    self.show_file_name.replace(show_file_name);
    self.update_title();
  }

  pub fn get_fullpath(&self) -> Option<String> {
    self.fullpath.borrow().clone()
  }

  pub fn connect_title_changed<F: Fn(&Self, &str) + 'static>(&self, f: F) {
    self.signal_title_changed.borrow_mut().replace(Box::new(f));
  }

  fn update_title(&self) {
    if let Some(callback) = self.signal_title_changed.borrow().as_ref() {
      if let Some(fullpath) = self.fullpath.borrow().as_ref() {
        let title = self.get_title(fullpath);
        callback(self, &title);
      }
    }
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
