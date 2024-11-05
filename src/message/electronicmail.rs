/* ElectronicMail.rs
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
use crate::message::{attachment::Attachment, message::MessageParser};
use base64::{engine::general_purpose, Engine};
use gmime::{
  glib, prelude::Cast, traits::{
    ContentTypeExt, DataWrapperExt, MessageExt, ObjectExt, ParserExt, PartExt, StreamExt, StreamMemExt
  }, InternetAddressExt, InternetAddressList, InternetAddressListExt, Message, Parser, Part, Stream, StreamFs, StreamMem
};
use nipper::Document;
use std::{error::Error, fs};

#[allow(unused_variables, dead_code)]
const O_RDONLY: i32 = 0;
#[allow(unused_variables, dead_code)]
pub const O_WRONLY: i32 = 1;
#[allow(unused_variables, dead_code)]
pub const O_RDWR: i32 = 2;
#[allow(unused_variables, dead_code)]
pub const O_CREAT: i32 = 100;
#[allow(unused_variables, dead_code)]
const INVALID_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

#[derive(Debug, Default, Clone)]
pub struct ElectronicMail {
  file: String,
  pub from: String,
  pub to: String,
  pub date: String,
  pub subject: String,
  pub body_html: Option<String>,
  pub body_text: Option<String>,
  pub attachments: Vec<Attachment>,
}

impl ElectronicMail {
  pub fn new(file: &str) -> ElectronicMail {
    ElectronicMail {
      file: file.to_string(),
      from: String::new(),
      to: String::new(),
      subject: String::new(),
      body_html: None,
      body_text: None,
      date: String::new(),
      attachments: vec![],
    }
  }

  fn merge_to(&self, message: &Message) -> InternetAddressList {
    let list = InternetAddressList::new();

    if let Some(to) = message.to() {
      for i in 0..to.length() {
        list.add(&to.address(i).unwrap());
      }
    }

    if let Some(cc) = message.cc() {
      for i in 0..cc.length() {
        list.add(&cc.address(i).unwrap());
      }
    }

    if let Some(bcc) = message.bcc() {
      for i in 0..bcc.length() {
        list.add(&bcc.address(i).unwrap());
      }
    }
    list
  }

  fn internet_list(&self, list: &InternetAddressList) -> String {
    let mut addresses: Vec<String> = Vec::new(); // Crée un vecteur vide de String

    for i in 0..list.length() {
      if let Some(addr) = list.address(i) {
        if let Some(address) = InternetAddressExt::to_string(&addr, None, false) {
          addresses.push(address.to_string());
        }
      }
    }
    addresses.join(", ")
  }

  fn parse_body(&mut self, message: &Message) {
    let mut html: Option<String> = None;
    message.foreach(|_, current| {
      log::debug!("part() => {:?}", current.content_id());
      if let Some(part) = current.dynamic_cast_ref::<Part>() {
        if part.is_attachment() {
          self.add_attachment(part);
        } else {
          // Note is_attachment() is false for inline (cid)
          if let Some(content_type) = part.content_type() {
            if content_type.is_type("text", "html") {
              html = Some(self.get_content(part));
            } else if content_type.is_type("text", "plain") {
              self.body_text = Some(self.get_content(part));
            } else {
              self.add_attachment(part);
            }
          }
        }
      }
    });
    if let Some(html) = html {
      self.body_html = Some(self.integrate_cid(&html));
      // for debugging parsed html
      // self.write_debug_html();
    }
  }

  #[allow(dead_code)]
  #[cfg(debug_assertions)]
  fn write_debug_html(&self) {
    if let Some(body) = &self.body_html {
      fs::write("body.html", &body).unwrap_or_else(|err| {
        log::error!("Failed to write body.html : {}", err);
      });
    }
  }

  fn get_attachment(&self, part: &Part) -> Option<Attachment> {
    let mut content_id: String = "none".to_string();
    let mut mime_type: Option<String> = None;
    if let Some(id) = part.content_id() {
      content_id = id.to_string();
    }
    if let Some(file) = part.filename() {
      let filename = file.to_string();
      if let Some(content_type) = part.content_type() {
        if let Some(parameter) = content_type.mime_type() {
          mime_type = Some(parameter.to_string());
        }
        if let Some(content) = part.content() {
          let stream = StreamMem::new();
          content.write_to_stream(&stream);
          let body = stream.byte_array().unwrap().to_vec();
          stream.close();

          return Some(Attachment {
            content_id,
            filename,
            mime_type,
            body,
          });
        }
      }
    }
    None
  }

  // It seems that gmime-rs has a memory free bug with g_mime_message_get_date()
  fn my_mime_message_get_date(e: &Message) -> Option<String> {
    let date: Option<glib::DateTime> = unsafe {
      glib::translate::from_glib_none(gmime::ffi::g_mime_message_get_date(
        glib::translate::ToGlibPtr::to_glib_none(&e).0,
      ))
    };
    let fmt_date: Option<String> = if let Some(date) = date {
      match date.format("%Y-%m-%d %H:%M:%S") {
        Ok(f) => Some(f.into()),
        Err(_) => None,
      }
    } else {
      None
    };
    fmt_date
  }

  fn latin1_to_string(s: &[u8]) -> String {
    s.iter().map(|&c| c as char).collect()
  }

  fn is_latin1(s: Option<glib::GString>) -> bool {
    if let Some(s) = s {
      if s.to_lowercase() == "iso-8859-1" {
        return true;
      }
    }
    false
  }

  fn integrate_cid(&self, body: &str) -> String {
    let document = Document::from(body);
    document.select("img").iter().for_each(|mut node| {
      if let Some(src) = node.attr("src") {
        if src.starts_with("cid:") {
          let cid = src.split_at(4).1;
          log::debug!("Found CID => {}", cid);
          if let Some(attachment) = self.attachments.iter().find(|a| a.content_id == cid) {
            log::debug!("Found CID Attachment => {}", attachment.filename);
            if let Some(mime_type) = attachment.mime_type.as_deref() {
              let b64 = general_purpose::STANDARD.encode(&attachment.body);
              log::debug!("Found CID with mime type => {}", mime_type);
              node.set_attr("src", &format!("data:{};base64,{}", mime_type, &b64));
            }
          }
        }
      }
    });
    document.html().to_string()
  }

  fn get_content(&self, part: &Part) -> String {
    let mut charset: Option<glib::GString> = None;

    log::debug!(
      "get_content() => part.content_type() {:?}",
      part.content_type()
    );
    log::debug!(
      "get_content() => part.content_encoding() {:?}",
      part.content_encoding()
    );
    log::debug!(
      "get_content() => part.content_disposition() {:?}",
      part.content_disposition()
    );

    if let Some(content_type) = part.content_type() {
      charset = content_type.parameter("charset");
    }

    if let Some(content) = part.content() {
      let stream = StreamMem::new();
      let size = content.write_to_stream(&stream) as u32;

      if size > 0 {
        let array: Vec<u8> = stream.byte_array().unwrap().to_vec();

        if ElectronicMail::is_latin1(charset) {
          log::debug!("get_content() ISO-8859-1");
          return ElectronicMail::latin1_to_string(&array);
        } else if let Some(body) = String::from_utf8(array).ok() {
          log::debug!("get_content() UTF8");
          return body;
        } else {
          log::debug!("get_content() FAILED => to convert to string");
        }
      } else {
        log::debug!("get_content() FAILED => size");
      }
    } else {
      log::debug!("get_content() FAILED => part.content()");
    }
    String::new()
  }

  fn add_attachment(&mut self, part: &Part) {
    if let Some(attachment) = self.get_attachment(part) {
      log::debug!(
        "add_attachment() => added attachment => {}",
        attachment.filename
      );
      self.attachments.push(attachment);
    } else {
      log::error!(
        "add_attachment() => no attachment => {:?}",
        part.content_id()
      );
    }
  }
}

impl Drop for ElectronicMail {
  fn drop(&mut self) {
    log::debug!("Drop ElectronicMail()");
    MessageParser::cleanup();
  }
}

#[cfg(test)]
mod tests {
  use crate::message::{electronicmail::ElectronicMail, message::Message};
  use std::{error::Error, path::Path};

  #[test]
  fn test_sample() -> Result<(), Box<dyn Error>> {
    let mut parser = ElectronicMail::new("sample.eml");
    parser.parse()?;
    assert_eq!(parser.from, "John Doe <john@moon.space>");
    assert_eq!(parser.to, "Lucas <lucas@mercure.space>");
    assert_eq!(parser.subject, "Lorem ipsum");
    assert_eq!(parser.date, "2024-10-23 12:27:21");
    assert_eq!(parser.attachments.len(), 1);
    let attachment = &parser.attachments[0];
    assert_eq!(attachment.filename, "Deus_Gnome.png");
    assert_eq!(attachment.content_id, "ii_m2lqbrhv0");
    assert_eq!(attachment.mime_type.as_ref().unwrap(), "image/png");
    let _name = attachment.write_to_tmp()?;
    let _file = Path::new(&_name);
    println!("file => {:?}", _file);
    assert!(_file.is_file());

    Ok(())
  }

  #[test]
  fn test_sample_google() -> Result<(), Box<dyn Error>> {
    let mut parser = ElectronicMail::new("tests/test-google.eml");
    parser.parse()?;
    assert_eq!(parser.from, "Bill Jncjkq <jncjkq@gmail.com>");
    assert_eq!(parser.to, "bookmarks@jncjkq.net");
    assert_eq!(parser.subject, "Test");
    assert_eq!(parser.date, "2011-05-11 13:27:12");
    assert_eq!(parser.attachments.len(), 1);
    let attachment = &parser.attachments[0];
    assert_eq!(attachment.filename, "bookmarks-really-short.html");
    assert_eq!(attachment.content_id, "none");
    assert_eq!(attachment.mime_type.as_ref().unwrap(), "text/html");

    Ok(())
  }

  #[test]
  fn test_sample_text() -> Result<(), Box<dyn Error>> {
    let mut parser = ElectronicMail::new("tests/text.eml");
    parser.parse()?;
    assert_eq!(parser.from, "John Doe <john@moon.space>");
    assert_eq!(parser.to, "Lucas <lucas@mercure.space>");
    assert_eq!(parser.subject, "Lorem ipsum");
    assert_eq!(parser.date, "2024-10-23 12:27:21");
    assert_ne!(parser.body_text, None);
    assert_eq!(parser.body_html, None);
    assert_eq!(parser.attachments.len(), 0);

    Ok(())
  }
  #[test]
  fn test_sample_html() -> Result<(), Box<dyn Error>> {
    let mut parser = ElectronicMail::new("tests/html.eml");
    parser.parse()?;
    assert_eq!(parser.from, "John Doe <john@moon.space>");
    assert_eq!(parser.to, "Lucas <lucas@mercure.space>");
    assert_eq!(parser.subject, "Lorem ipsum");
    assert_eq!(parser.date, "2024-10-23 12:27:21");
    assert_eq!(parser.body_text, None);
    assert_ne!(parser.body_html, None);
    assert_eq!(parser.attachments.len(), 0);

    Ok(())
  }

  #[test]
  fn test_sample_php() -> Result<(), Box<dyn Error>> {
    let mut parser = ElectronicMail::new("tests/test-php.eml");
    parser.parse()?;
    assert_eq!(parser.from, "mlemos <mlemos@acm.org>");
    assert_eq!(parser.to, "Manuel Lemos <mlemos@linux.local>");
    assert_eq!(
      parser.subject,
      "Testing Manuel Lemos' MIME E-mail composing and sending PHP class: HTML message"
    );
    assert_eq!(parser.date, "2005-04-30 19:28:29");
    assert_ne!(parser.body_text, None);
    assert_ne!(parser.body_html, None);
    assert_eq!(parser.attachments.len(), 3);
    assert_eq!(parser.attachments[0].filename, "logo.gif");
    assert_eq!(
      parser.attachments[0].mime_type.as_ref().unwrap(),
      "image/gif"
    );
    assert_eq!(
      parser.attachments[0].content_id,
      "ae0357e57f04b8347f7621662cb63855.gif"
    );
    assert_eq!(parser.attachments[0].body.len(), 1195);
    assert_eq!(parser.attachments[1].filename, "background.gif");
    assert_eq!(
      parser.attachments[1].mime_type.as_ref().unwrap(),
      "image/gif"
    );
    assert_eq!(
      parser.attachments[1].content_id,
      "4c837ed463ad29c820668e835a270e8a.gif"
    );
    assert_eq!(parser.attachments[1].body.len(), 3265);
    assert_eq!(parser.attachments[2].filename, "attachment.txt");
    assert_eq!(
      parser.attachments[2].mime_type.as_ref().unwrap(),
      "text/plain"
    );
    assert_eq!(parser.attachments[2].content_id, "none");
    assert_eq!(parser.attachments[2].body.len(), 64);
    Ok(())
  }
}

impl super::message::Message for ElectronicMail {
  fn parse(&mut self) -> Result<(), Box<dyn Error>> {
    let stream: Stream = StreamFs::open(&self.file, O_RDONLY, 0644)?;
    let parser = Parser::with_stream(&stream);
    let message = parser.construct_message(None);

    if let Some(eml) = &message {
      if let Some(from) = &eml.from() {
        self.from = self.internet_list(from);
      }
      self.to = self.internet_list(&self.merge_to(&eml));
      if let Some(subject) = &eml.subject() {
        self.subject = subject.to_string();
      }
      if let Some(date) = ElectronicMail::my_mime_message_get_date(&eml) {
        self.date = date;
      }
      self.parse_body(&eml);
    }
    stream.close();

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
    self.body_html.clone()
  }

  fn body_text(&self) -> Option<String> {
    self.body_text.clone()
  }
}
