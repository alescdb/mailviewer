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

use crate::gio;
use gio::prelude::*;

use msg_parser::Outlook;

use super::attachment::Attachment;
use super::message::Message;
use crate::message::message::MessageParser;

#[derive(Debug, Default, Clone)]
pub struct OutlookMessage {
  data: Vec<u8>,
  pub from: String,
  pub to: String,
  pub date: String,
  pub subject: String,
  pub body: Option<String>,
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
}

impl Message for OutlookMessage {
  fn parse(&mut self, cancellable: Option<&gio::Cancellable>) -> Result<(), Box<dyn Error>> {
    let outlook = Outlook::from_slice(&self.data)?;

    if let Some(cancellable) = cancellable {
      cancellable.set_error_if_cancelled()?;
    }

    self.from = OutlookMessage::person_to_string(&outlook.sender);
    self.to = OutlookMessage::person_list_to_string(&outlook.to);
    self.subject = outlook.subject;
    self.date = outlook.headers.date;
    self.body = Some(outlook.body.clone());

    for i in 0..outlook.attachments.capacity() {
      if let Some(cancellable) = cancellable {
        cancellable.set_error_if_cancelled()?;
      }

      let att = &outlook.attachments[i];
      self.attachments.push(Attachment {
        filename: att.file_name.clone(),
        content_id: att.file_name.clone(), // Uuid::new_v4().simple().to_string(),
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
    None
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
}
