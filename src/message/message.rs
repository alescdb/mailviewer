use std::{error::Error, fs, path::PathBuf};

use super::attachment::Attachment;
use crate::{
  config::APP_NAME,
  message::{electronicmail::ElectronicMail, outlook::OutlookMessage},
};
use lazy_static::lazy_static;
use uuid::Uuid;

lazy_static! {
  pub static ref TEMP_FOLDER: PathBuf = {
    let mut path = PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap());
    let uuid = Uuid::new_v4().simple().to_string();
    path.push(APP_NAME);
    path.push(uuid);
    path
  };
}
pub trait Message {
  fn parse(&mut self) -> Result<(), Box<dyn Error>>;
  fn from(&self) -> String;
  fn to(&self) -> String;
  fn subject(&self) -> String;
  fn date(&self) -> String;
  fn attachments(&self) -> Vec<Attachment>;
  fn body_html(&self) -> Option<String>;
  fn body_text(&self) -> Option<String>;
}

pub struct MessageParser {
  parser: Box<dyn Message>,
}

impl MessageParser {
  pub fn new(file: &str) -> Self {
    // assert!(file.ends_with(".eml") || file.ends_with(".msg"));
    Self {
      parser: if file.ends_with(".msg") {
        Box::new(OutlookMessage::new(file))
      } else {
        Box::new(ElectronicMail::new(file))
      },
    }
  }

  pub fn cleanup() {
    log::debug!("MessageParser::cleanup()");
    if TEMP_FOLDER.exists() {
      log::debug!("remove_dir_all({:?})", TEMP_FOLDER.to_str());
      fs::remove_dir_all(TEMP_FOLDER.to_path_buf()).unwrap_or_else(|err| {
        log::error!("Error while removing {:?} : {}", TEMP_FOLDER.to_str(), err);
      });
    }
  }
}

impl Drop for MessageParser {
  fn drop(&mut self) {
    log::warn!("MessageParser::drop()");
    Self::cleanup();
  }
}

impl Message for MessageParser {
  fn parse(&mut self) -> Result<(), Box<dyn Error>> {
    Ok(self.parser.parse()?)
  }

  fn from(&self) -> String {
    self.parser.from()
  }

  fn to(&self) -> String {
    self.parser.to()
  }

  fn subject(&self) -> String {
    self.parser.subject()
  }

  fn date(&self) -> String {
    self.parser.date()
  }

  fn attachments(&self) -> Vec<Attachment> {
    self.parser.attachments()
  }

  fn body_html(&self) -> Option<String> {
    self.parser.body_html()
  }

  fn body_text(&self) -> Option<String> {
    self.parser.body_text()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_sample() {
    let mut message = MessageParser::new("sample.eml");
    message.parse().unwrap();
    assert_eq!(message.from(), "John Doe <john@moon.space>");
    assert_eq!(message.to(), "Lucas <lucas@mercure.space>");
    assert_eq!(message.subject(), "Lorem ipsum");
    assert_eq!(message.date(), "2024-10-23 12:27:21");
    assert_eq!(message.attachments().len(), 1);
    let attachment = &message.attachments()[0];
    assert_eq!(attachment.filename, "Deus_Gnome.png");
    assert_eq!(attachment.content_id, "ii_m2lqbrhv0");
    assert_eq!(attachment.mime_type.as_ref().unwrap(), "image/png");
  }
}
