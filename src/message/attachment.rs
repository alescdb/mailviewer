/* attachment.rs
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
use std::fmt;

use super::message::TEMP_FOLDER;
use crate::gio::prelude::*;
use crate::{gio, glib};

const DEFAULT_FILENAME: &str = "attachment";

#[derive(Debug, Clone)]
pub struct Attachment {
  pub filename: String,
  pub content_id: String,
  pub body: Vec<u8>,
  pub mime_type: Option<String>,
}

impl Attachment {
  /// The file name comes from the message, so it can contain anything, including
  /// path separators and dot segments. gio::File::child() resolves those, so
  /// writing to `filename` directly can escape the target directory.
  pub fn safe_filename(&self) -> String {
    let name = self
      .filename
      .rsplit(['/', '\\'])
      .next()
      .unwrap_or("");
    let name: String = name.chars().filter(|c| !c.is_control()).collect();
    let name = name.trim();

    if name.is_empty() || name == "." || name == ".." {
      return DEFAULT_FILENAME.to_string();
    }
    name.to_string()
  }

  pub async fn write_to_tmp(&self) -> Result<gio::File, Box<dyn Error>> {
    let tmp = gio::File::for_path(TEMP_FOLDER.to_str().unwrap());
    if file_exists(&tmp).await.is_ok_and(|v| !v) {
      log::debug!("create_dir({:?})", tmp);
      tmp.make_directory_future(glib::Priority::default()).await?;
    }
    let tmp = tmp.child(self.safe_filename());
    log::debug!("write_to_tmp({:?})", tmp);
    self.write_to_file(&tmp).await?;
    Ok(tmp)
  }

  pub async fn write_to_file(&self, file: &gio::File) -> Result<(), Box<dyn Error>> {
    let io_stream = if file_exists(file).await.is_ok_and(|v| v) {
      file
        .open_readwrite_future(glib::Priority::default())
        .await?
    } else {
      file
        .create_readwrite_future(
          gio::FileCreateFlags::REPLACE_DESTINATION,
          glib::Priority::default(),
        )
        .await?
    };

    let output_stream = io_stream.output_stream();
    let write_res = output_stream
      .write_future(glib::Bytes::from(&self.body), glib::Priority::DEFAULT)
      .await;

    io_stream.close_future(glib::Priority::default()).await?;

    match write_res {
      Ok((_, written)) => {
        if written != self.body.len() {
          return Err(
            format!(
              "Failed to write {} to file {}: only {} of {} bytes have been written",
              self,
              file.peek_path().unwrap_or_default().display(),
              written,
              self.body.len()
            )
            .into(),
          );
        }

        Ok(())
      }
      Err((_, e)) => Err(Box::new(e)),
    }
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

#[cfg(test)]
mod tests {
  use super::*;

  fn attachment(filename: &str) -> Attachment {
    Attachment {
      filename: filename.to_string(),
      content_id: String::new(),
      body: vec![],
      mime_type: None,
    }
  }

  #[test]
  fn safe_filename_keeps_regular_names() {
    assert_eq!(attachment("Deus_Gnome.png").safe_filename(), "Deus_Gnome.png");
    assert_eq!(attachment("état des lieux.pdf").safe_filename(), "état des lieux.pdf");
  }

  #[test]
  fn safe_filename_strips_directories() {
    assert_eq!(attachment("../../.bashrc").safe_filename(), ".bashrc");
    assert_eq!(attachment("/etc/passwd").safe_filename(), "passwd");
    assert_eq!(attachment("..\\..\\evil.exe").safe_filename(), "evil.exe");
    assert_eq!(attachment("a/b/c.png").safe_filename(), "c.png");
  }

  #[test]
  fn safe_filename_falls_back_when_nothing_is_left() {
    assert_eq!(attachment("").safe_filename(), DEFAULT_FILENAME);
    assert_eq!(attachment("..").safe_filename(), DEFAULT_FILENAME);
    assert_eq!(attachment("/").safe_filename(), DEFAULT_FILENAME);
    assert_eq!(attachment("evil/").safe_filename(), DEFAULT_FILENAME);
  }

  #[test]
  fn safe_filename_strips_control_characters() {
    assert_eq!(attachment("a\nb\tc.png").safe_filename(), "abc.png");
  }
}

async fn file_exists(file: &gio::File) -> Result<bool, Box<dyn Error>> {
  match file
    .query_info_future(
      gio::FILE_ATTRIBUTE_STANDARD_NAME,
      gio::FileQueryInfoFlags::NONE,
      glib::Priority::default(),
    )
    .await
  {
    Ok(_) => Ok(true),
    Err(e) => {
      if !e.matches(gio::IOErrorEnum::NotFound) {
        return Err(Box::new(e));
      }

      Ok(false)
    }
  }
}
