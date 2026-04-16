/* outlook.rs
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

use gio::prelude::*;
use msg_parser::Outlook;

use super::attachment::Attachment;
use super::message::Message;
use crate::gio;
use crate::message::message::MessageParser;

#[derive(Debug, Default, Clone)]
pub struct OutlookMessage {
  data: Vec<u8>,
  pub from: String,
  pub to: String,
  pub date: String,
  pub subject: String,
  pub body: Option<String>,
  pub html: Option<String>,
  pub attachments: Vec<Attachment>,
}

impl OutlookMessage {
  pub fn new(data: Vec<u8>) -> Self {
    Self {
      data: data,
      from: String::new(),
      to: String::new(),
      date: String::new(),
      subject: String::new(),
      body: None,
      html: None,
      attachments: vec![],
    }
  }

  fn person_to_string(person: &msg_parser::Person) -> String {
    format!("{} <{}>", person.name, person.email)
  }

  fn person_list_to_string(persons: &Vec<msg_parser::Person>) -> String {
    persons
      .iter()
      .map(|person| OutlookMessage::person_to_string(person))
      .collect::<Vec<String>>()
      .join(", ")
  }

  /* some msg fields contains null bytes and gtk4 components can't handle them */
  fn clean_string(mut value: String) -> String {
    value.retain(|c| c != '\0');
    value
  }
}

impl Message for OutlookMessage {
  fn parse(&mut self, cancellable: Option<&gio::Cancellable>) -> Result<(), Box<dyn Error>> {
    let outlook = Outlook::from_slice(&self.data)?;

    if let Some(cancellable) = cancellable {
      cancellable.set_error_if_cancelled()?;
    }

    self.from = Self::clean_string(OutlookMessage::person_to_string(&outlook.sender));
    self.to = Self::clean_string(OutlookMessage::person_list_to_string(&outlook.to));
    self.subject = Self::clean_string(outlook.subject);
    self.date = Self::clean_string(outlook.headers.date);
    self.body = if outlook.body.is_empty() {
      None
    } else {
      Some(Self::clean_string(outlook.body.clone()))
    };
    self.html = if outlook.html.is_empty() {
      None
    } else {
      match hex::decode(&outlook.html) {
        Ok(bytes) => Some(
          String::from_utf8(bytes).unwrap_or_else(|_| Self::clean_string(outlook.html.clone())),
        ),
        Err(e) => {
          log::error!("Failed to decode Hex -> HTML: {}", e);
          Some(Self::clean_string(outlook.html.clone()))
        }
      }
    };

    // log::debug!("[DEBUG] OUTLOOK HTML: {}", &outlook.html);
    // log::debug!("[DEBUG] OUTLOOK HTML Final: {:?}", &self.html);

    for att in &outlook.attachments {
      if let Some(cancellable) = cancellable {
        cancellable.set_error_if_cancelled()?;
      }

      self.attachments.push(Attachment {
        filename: Self::clean_string(att.file_name.clone()),
        content_id: Self::clean_string(att.file_name.clone()), // Uuid::new_v4().simple().to_string(),
        body: hex::decode(&att.payload)?,
        mime_type: Some(att.mime_tag.clone()),
      });
    }

    Ok(())
  }

  fn from(&self) -> String {
    self.from.clone()
  }

  fn to(&self) -> String {
    self.to.clone()
  }

  fn subject(&self) -> String {
    self.subject.clone()
  }

  fn date(&self) -> String {
    self.date.clone()
  }

  fn attachments(&self) -> Vec<Attachment> {
    self.attachments.clone()
  }

  fn body_html(&self) -> Option<String> {
    self.html.clone()
  }

  fn body_text(&self) -> Option<String> {
    self.body.clone()
  }
}

impl Drop for OutlookMessage {
  fn drop(&mut self) {
    log::warn!("OutlookMessage::drop()");
    MessageParser::cleanup();
  }
}

#[cfg(test)]
mod tests {
  use std::error::Error;
  use std::fs;

  use crate::message::message::Message;
  use crate::message::outlook::OutlookMessage;

  #[test]
  fn test_outlook() -> Result<(), Box<dyn Error>> {
    let mut parser = OutlookMessage::new(fs::read("sample.msg").unwrap());
    parser.parse(None)?;
    assert_eq!(parser.from, "John Doe <john@moon.space>");
    assert_eq!(parser.to, "Lucas <lucas@mercure.space>");
    assert_eq!(parser.subject, "Lorem ipsum");
    assert_eq!(parser.date, "");
    assert_eq!(parser.attachments.len(), 3);
    assert_eq!(parser.attachments[0].filename, "image001.png");
    assert!(parser.body.clone().unwrap().contains("Hello Lucas"));
    assert_eq!(
      parser.attachments[0].mime_type.clone().unwrap(),
      "image/png"
    );
    Ok(())
  }

  #[test]
  fn clean_string_bytes() {
    assert_eq!(OutlookMessage::clean_string("a\0b\0c".to_string()), "abc");
  }
}
