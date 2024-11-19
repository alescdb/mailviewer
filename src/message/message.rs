/* message.rs
 *
 * Copyright 2024 Alexandre Del Bigio
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use lazy_static::lazy_static;
use uuid::Uuid;

use super::attachment::Attachment;
use crate::config::APP_NAME;
use crate::message::electronicmail::ElectronicMail;
use crate::message::outlook::OutlookMessage;

lazy_static! {
  pub static ref TEMP_FOLDER: PathBuf = {
    let mut path = PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap());
    let uuid = Uuid::new_v4().simple().to_string();
    path.push(APP_NAME);
    if path.exists() == false {
      if let Err(e) = fs::create_dir(path.clone()) {
        log::error!("Error while creating {:?} : {}", path.to_str(), e);
      }
    }
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
      parser: if file.to_lowercase().ends_with(".msg") {
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
    log::debug!("MessageParser::drop()");
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
  fn test_sample_eml() {
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

  #[test]
  fn test_sample_msg() {
    let mut message = MessageParser::new("sample.msg");
    message.parse().unwrap();
    assert_eq!(message.from(), "John Doe <john@moon.space>");
    assert_eq!(message.to(), "Lucas <lucas@mercure.space>");
    assert_eq!(message.subject(), "Lorem ipsum");
    assert_eq!(message.date(), "");
    assert_eq!(message.attachments().len(), 3);
    let attachment = &message.attachments()[0];
    assert_eq!(attachment.filename, "image001.png");
    assert_eq!(attachment.content_id, "image001.png"); // same as filename
    assert_eq!(attachment.mime_type.as_ref().unwrap(), "image/png");
  }
}
