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
use gmime::glib;
use msg_parser::Outlook;

use super::attachment::Attachment;
use super::message::Message;
use crate::core::message::headers::Header;
use crate::core::message::message::{MessageParser, Protection};

#[derive(Debug, Default, Clone)]
pub struct OutlookMessage {
  data: Vec<u8>,
  pub from: String,
  pub to: String,
  pub date: Option<gmime::DateTime>,
  pub subject: String,
  pub body: Option<String>,
  pub html: Option<String>,
  pub attachments: Vec<Attachment>,
  pub headers: Vec<Header>,
}

impl OutlookMessage {
  pub fn new(data: Vec<u8>) -> Self {
    Self {
      data,
      from: String::new(),
      to: String::new(),
      date: None,
      subject: String::new(),
      body: None,
      html: None,
      attachments: vec![],
      headers: vec![],
    }
  }

  fn get_date(&self, dstr: &str) -> Option<gmime::DateTime> {
    let date = Self::clean_string(dstr.to_string());
    unsafe {
      glib::translate::from_glib_full(gmime::ffi::g_mime_utils_header_decode_date(
        glib::translate::ToGlibPtr::to_glib_none(&date).0,
      ))
    }
  }

  fn person_to_string(person: &msg_parser::Person) -> String {
    format!("{} <{}>", person.name, person.email)
  }

  fn person_list_to_string(persons: &[msg_parser::Person]) -> String {
    persons
      .iter()
      .map(OutlookMessage::person_to_string)
      .collect::<Vec<String>>()
      .join(", ")
  }

  /* some msg fields contains null bytes and gtk4 components can't handle them */
  fn clean_string(mut value: String) -> String {
    value.retain(|c| c != '\0');
    value
  }

  fn parse_headers(outlook: &Outlook) -> Vec<super::headers::Header> {
    Self::parse_raw_headers(&outlook.headers.raw)
  }

  fn parse_raw_headers(raw: &str) -> Vec<Header> {
    let mut headers = Vec::new();
    let mut current: Option<Header> = None;

    for line in raw.lines() {
      if line.starts_with([' ', '\t']) {
        if let Some(header) = &mut current {
          header.value.push(' ');
          header.value.push_str(line.trim());
        }
        continue;
      }

      if let Some(header) = current.take() {
        headers.push(header);
      }

      if let Some((name, value)) = line.split_once(':') {
        current = Some(Header {
          name: name.trim().to_string(),
          value: Self::clean_string(value.trim().to_string()),
        });
      }
    }

    if let Some(header) = current {
      headers.push(header);
    }

    headers
  }
}

impl Message for OutlookMessage {
  fn parse(&mut self, cancellable: Option<&gio::Cancellable>) -> Result<(), Box<dyn Error>> {
    let outlook = Outlook::from_slice(&self.data)?;

    if let Some(cancellable) = cancellable {
      cancellable.set_error_if_cancelled()?;
    }

    self.headers = Self::parse_headers(&outlook);
    self.from = Self::clean_string(OutlookMessage::person_to_string(&outlook.sender));
    self.to = Self::clean_string(OutlookMessage::person_list_to_string(&outlook.to));
    self.subject = Self::clean_string(outlook.subject);
    self.date = self.get_date(&outlook.headers.date);
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
        filename: Self::clean_string(att.long_file_name.clone()),
        content_id: Self::clean_string(att.long_file_name.clone()), // Uuid::new_v4().simple().to_string(),
        body: std::sync::Arc::from(hex::decode(&att.payload)?.as_slice()),
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
    MessageParser::to_local_date(&self.date)
  }

  fn attachments(&self) -> Vec<Attachment> {
    self.attachments.clone()
  }

  fn body_html(&self) -> Option<String> {
    self.html.clone()
  }

  fn protection(&self) -> Protection {
    // Outlook keeps a signed or encrypted message as an attached blob rather
    // than in the mime structure, so nothing is claimed here.
    Protection::None
  }

  fn body_text(&self) -> Option<String> {
    self.body.clone()
  }

  fn headers(&self) -> Vec<super::headers::Header> {
    self.headers.clone()
  }
}

#[cfg(test)]
mod tests {
  use std::error::Error;
  use std::fs;

  use crate::core::message::message::Message;
  use crate::core::message::outlook::OutlookMessage;

  #[test]
  fn test_outlook() -> Result<(), Box<dyn Error>> {
    let mut parser = OutlookMessage::new(fs::read("sample.msg").unwrap());
    parser.parse(None)?;
    assert_eq!(parser.from, "John Doe <john@moon.space>");
    assert_eq!(parser.to, "Lucas <lucas@mercure.space>");
    assert_eq!(parser.subject, "Lorem ipsum");
    assert!(parser.date.is_none());
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

  #[test]
  fn parse_transport_headers() {
    assert_eq!(
      OutlookMessage::parse_raw_headers(
        "Subject: Test\r\nX-Long: first\r\n\tsecond\r\nInvalid\r\n"
      ),
      vec![
        super::super::headers::Header {
          name: "Subject".to_string(),
          value: "Test".to_string(),
        },
        super::super::headers::Header {
          name: "X-Long".to_string(),
          value: "first second".to_string(),
        },
      ]
    );
  }
}
