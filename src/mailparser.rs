/* mailparser.rs
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
use base64::{engine::general_purpose, Engine};
use gmime::{
  glib,
  prelude::Cast,
  traits::{
    ContentTypeExt, DataWrapperExt, MessageExt, ObjectExt, ParserExt, PartExt, StreamExt,
    StreamMemExt,
  },
  InternetAddressExt, InternetAddressList, InternetAddressListExt, Message, Parser, Part, Stream,
  StreamFs, StreamMem,
};
use nipper::Document;
use std::{error::Error, fmt, fs, path::PathBuf};

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

#[derive(Debug, Clone)]
pub struct Attachment {
  temp: String,
  pub filename: String,
  pub content_id: String,
  pub body: Vec<u8>,
  pub mime_type: Option<String>,
}

impl Attachment {
  pub fn write_to_tmp(&self) -> Result<String, Box<dyn Error>> {
    self.write_to_file(&self.temp)?;
    Ok(self.temp.to_string())
  }

  pub fn write_to_file(&self, file: &str) -> std::io::Result<()> {
    fs::write(&file, &self.body)
  }
}

impl fmt::Display for Attachment {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(
      f,
      "Attachment(content_id: {}, filename: {}, mime_type: {})",
      self.content_id,
      self.filename,
      self.mime_type.as_deref().unwrap_or("None")
    )
  }
}

#[derive(Debug, Default)]
pub struct MailParser {
  file: String,
  pub from: String,
  pub to: String,
  pub date: String,
  pub subject: String,
  pub body_html: Option<String>,
  pub body_text: Option<String>,
  pub attachments: Vec<Attachment>,
}

impl Drop for MailParser {
  fn drop(&mut self) {
    unsafe {
      gmime::ffi::g_mime_shutdown();
    }

    let tmp = MailParser::get_temp_folder();
    if tmp.exists() {
      log::debug!("remove_dir_all({:?})", &tmp);
      fs::remove_dir_all(&tmp).unwrap_or_else(|err| {
        log::error!("Error while removing {:?} : {}", tmp, err);
      });
    }
  }
}

impl MailParser {
  pub fn new(file: &str) -> MailParser {
    unsafe {
      gmime::ffi::g_mime_init();
    }
    MailParser {
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

  fn get_temp_folder() -> PathBuf {
    let pid = std::process::id();
    let mut path = std::env::temp_dir();
    path.push("mailviewer");
    path.push(pid.to_string());
    path
  }

  fn get_temp_name(file: &str) -> String {
    let mut path = MailParser::get_temp_folder();
    if path.exists() == false {
      log::debug!("create_dir_all({:?}) for {}", &path.to_str(), file);
      fs::create_dir_all(&path).unwrap();
    }
    path.push(file);
    path.to_str().unwrap().to_string()
  }

  pub fn parse(&mut self) -> Result<(), Box<dyn Error>> {
    log::debug!("[MailParser] FILE : {}", self.file);
    let stream: Stream = StreamFs::open(&self.file, O_RDONLY, 0644)?;
    let parser = Parser::with_stream(&stream);
    let message = parser.construct_message(None);

    log::debug!("[MailParser] EML MESSAGE {:?}", message);
    if let Some(eml) = &message {
      if let Some(from) = &eml.from() {
        self.from = self.internet_list(from);
      }
      self.to = self.internet_list(&self.merge_to(&eml));
      if let Some(subject) = &eml.subject() {
        self.subject = subject.to_string();
      }
      if let Some(date) = &eml.date() {
        self.date = match date.format("%Y-%m-%d %H:%M:%S") {
          Ok(s) => s.to_string(),
          Err(e) => e.to_string(),
        }
      }
      self.parse_body(&eml);
    } else {
      log::debug!("[MailParser] EML IS NULL !");
    }
    stream.close();
    log::debug!("[MailParser] EML FROM : '{}'", self.from);

    Ok(())
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
            temp: MailParser::get_temp_name(&filename),
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

        if MailParser::is_latin1(charset) {
          log::debug!("get_content() ISO-8859-1");
          return MailParser::latin1_to_string(&array);
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
