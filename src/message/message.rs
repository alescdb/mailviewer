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
use crate::gio::prelude::*;
use crate::message::electronicmail::ElectronicMail;
use crate::message::outlook::OutlookMessage;
use crate::{gio, glib};

const EML_MIME_TYPES: [&str; 1] = ["message/rfc822"];

const MSG_MIME_TYPES: [&str; 2] = ["application/vnd.ms-outlook", "application/x-ole-storage"];

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
  fn parse(&mut self, cancellable: Option<&gio::Cancellable>) -> Result<(), Box<dyn Error>>;
  fn from(&self) -> String;
  fn to(&self) -> String;
  fn subject(&self) -> String;
  fn date(&self) -> String;
  fn attachments(&self) -> Vec<Attachment>;
  fn body_html(&self) -> Option<String>;
  fn body_text(&self) -> Option<String>;
}

#[derive(PartialEq, Debug)]
#[repr(u8)]
pub enum MessageType {
  Eml = 0,
  Msg = 1,
}

pub struct MessageParser {
  parser: Box<dyn Message + Send>,
  #[allow(dead_code)]
  message_type: MessageType,
}

impl MessageParser {
  pub async fn new(
    file: &gio::File,
    cancellable: Option<&gio::Cancellable>,
  ) -> Result<Self, Box<dyn Error>> {
    let gio_type = Self::message_type(file).await.ok();
    let content = Self::message_content(file, cancellable).await?;
    let message_type = gio_type.unwrap_or_else(|| {
      let mt = Self::message_type_from_content(&content);
      log::debug!("Falling back to content-based detection: {:?}", mt);
      mt
    });
    Ok(Self {
      parser: if message_type == MessageType::Msg {
        Box::new(OutlookMessage::new(content))
      } else {
        Box::new(ElectronicMail::new(content))
      },
      message_type,
    })
  }

  pub fn supported_mime_types() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = Vec::with_capacity(EML_MIME_TYPES.len() + MSG_MIME_TYPES.len());
    v.extend(EML_MIME_TYPES.iter().copied());
    v.extend(MSG_MIME_TYPES.iter().copied());
    v
  }

  #[allow(dead_code)]
  async fn message_type(file: &gio::File) -> Result<MessageType, Box<dyn Error>> {
    let file_info = file
      .query_info_future(
        gio::FILE_ATTRIBUTE_STANDARD_CONTENT_TYPE.as_str(),
        gio::FileQueryInfoFlags::NONE,
        glib::Priority::DEFAULT,
      )
      .await?;

    let content_type = file_info.content_type().unwrap_or_default();
    let content_type = content_type.as_str();
    log::debug!(
      "MessageParser::message_type({}) content type: {}",
      file.peek_path().unwrap().display(),
      content_type
    );

    if EML_MIME_TYPES.contains(&content_type) {
      return Ok(MessageType::Eml);
    }

    if MSG_MIME_TYPES.contains(&content_type) {
      return Ok(MessageType::Msg);
    }

    Err(
      format!(
        "File {} has an unsupported content type: {}",
        file.peek_path().unwrap().display(),
        content_type
      )
      .into(),
    )
  }

  #[inline]
  fn message_type_from_content(content: &[u8]) -> MessageType {
    const OLE_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    if content.len() >= OLE_MAGIC.len() && &content[..OLE_MAGIC.len()] == OLE_MAGIC {
      MessageType::Msg
    } else {
      MessageType::Eml
    }
  }

  async fn message_content(
    file: &gio::File,
    cancellable: Option<&gio::Cancellable>,
  ) -> Result<Vec<u8>, Box<dyn Error>> {
    let input_stream = file.read_future(glib::Priority::DEFAULT).await?;

    let read_input_stream = async || -> Result<Vec<u8>, Box<dyn Error>> {
      let mut out: Vec<u8> = Vec::new();
      loop {
        if let Some(cancellable) = cancellable {
          cancellable.set_error_if_cancelled()?;
        }
        let buf = input_stream
          .read_bytes_future(8192, glib::Priority::DEFAULT)
          .await?;
        if buf.len() == 0 {
          break;
        }
        out.extend_from_slice(&buf);
      }
      Ok(out)
    };

    let input_stream_result = read_input_stream().await;
    input_stream.close_future(glib::Priority::DEFAULT).await?;

    input_stream_result
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
  fn parse(&mut self, cancellable: Option<&gio::Cancellable>) -> Result<(), Box<dyn Error>> {
    Ok(self.parser.parse(cancellable)?)
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
  use crate::{gio, utils};

  #[test]
  fn test_sample_eml() {
    let file = gio::File::for_path("sample.eml");

    utils::spawn_and_wait_new_ctx(async move {
      let mut message = MessageParser::new(&file, None).await.expect("File opened");
      message.parse(None).unwrap();
      assert_eq!(message.from(), "John Doe <john@moon.space>");
      assert_eq!(message.to(), "Lucas <lucas@mercure.space>");
      assert_eq!(message.subject(), "Lorem ipsum");
      assert_eq!(message.date(), "2024-10-23 12:27:21");
      assert_eq!(message.attachments().len(), 1);
      let attachment = &message.attachments()[0];
      assert_eq!(attachment.filename, "Deus_Gnome.png");
      assert_eq!(attachment.content_id, "ii_m2lqbrhv0");
      assert_eq!(attachment.mime_type.as_ref().unwrap(), "image/png");
    });
  }

  #[test]
  fn test_sample_msg() {
    let file = gio::File::for_path("sample.msg");

    utils::spawn_and_wait_new_ctx(async move {
      let mut message = MessageParser::new(&file, None).await.expect("File opened");

      message.parse(None).unwrap();
      assert_eq!(message.from(), "John Doe <john@moon.space>");
      assert_eq!(message.to(), "Lucas <lucas@mercure.space>");
      assert_eq!(message.subject(), "Lorem ipsum");
      assert_eq!(message.date(), "");
      assert_eq!(message.attachments().len(), 3);
      let attachment = &message.attachments()[0];
      assert_eq!(attachment.filename, "image001.png");
      assert_eq!(attachment.content_id, "image001.png"); // same as filename
      assert_eq!(attachment.mime_type.as_ref().unwrap(), "image/png");
    });
  }
}
