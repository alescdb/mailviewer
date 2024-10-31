use crate::{
  config::VERSION,
  mailparser::{Attachment, MailParser},
};
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

#[cfg(test)]
mod tests {
  use std::rc::Rc;
  use crate::mailservice::MailService;
  
  #[test]
  fn new_mail_service() {
    let service = MailService::new();
    
    assert!(service.parser.borrow().is_none());
    assert!(service.fullpath.borrow().is_none());
    assert_eq!(*service.show_file_name.borrow(), true);
  }

  #[test]
  fn open_mail_success() {
    let mail_service = MailService::new();
    let service = mail_service;
    let fullpath = "sample.eml";

    assert!(service.open_mail(fullpath).is_ok());
    assert_eq!(service.get_fullpath().unwrap(), fullpath.to_string());
    assert_eq!(service.get_from(), "John Doe <john@moon.space>");
    assert_eq!(service.get_to(), "Lucas <lucas@mercure.space>");
    assert_eq!(service.get_subject(), "Lorem ipsum");
    assert_eq!(service.get_date(), "2024-10-23 12:27:21");
  }

  #[test]
  fn open_mail_file_not_found() {
    let service = MailService::new();
    let result = service.open_mail("path/to/nonexistent.eml");

    assert!(result.is_err());
    assert_eq!(
      format!("{}", result.unwrap_err()),
      "File not found : path/to/nonexistent.eml"
    );
  }

  #[test]
  fn get_text() {
    let service = MailService::new();
    service.open_mail("sample.eml").unwrap();
    let text = service.get_text().unwrap();

    assert!(text.contains("Lorem ipsum dolor sit amet, consectetur adipiscing elit"));
  }

  #[test]
  fn get_html() {
    let service = MailService::new();
    service.open_mail("sample.eml").unwrap();
    let html = service.get_html().unwrap();

    assert!(html.contains("Hello Lucas,"));
  }

  #[test]
  fn get_attachments() {
    let service = MailService::new();
    
    service.open_mail("sample.eml").unwrap();
    let attachments = service.get_attachments();

    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0].filename, "Deus_Gnome.png");
  }

  #[test]
  fn update_title_with_show_file_name() {
    let service = MailService::new();
    service.open_mail("sample.eml").unwrap();
    service.set_show_file_name(true);
    assert_eq!(service.get_title("sample.eml"), "sample.eml");
  }

  #[test]
  fn update_title_without_show_file_name() {
    let service = MailService::new();
    service.set_show_file_name(false);
    assert_eq!(service.get_title("sample.eml"), format!("Mail Viewer v{}", crate::config::VERSION));
  }

  #[test]
  fn connect_title_changed() {
    let service = MailService::new();
    let title_changed_called = Rc::new(std::cell::RefCell::new(false));
    let title_changed_called_clone = Rc::clone(&title_changed_called);
    service.connect_title_changed(move |_, _| {
      *title_changed_called_clone.borrow_mut() = true;
    });
    service.open_mail("sample.eml").unwrap();
    service.set_show_file_name(false);
    assert!(*title_changed_called.borrow());
  }
}
