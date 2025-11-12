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

use crate::gio;
use crate::gio::prelude::*;
use crate::glib;

use super::message::TEMP_FOLDER;

#[derive(Debug, Clone)]
pub struct Attachment {
  pub filename: String,
  pub content_id: String,
  pub body: Vec<u8>,
  pub mime_type: Option<String>,
}

impl Attachment {
  pub async fn write_to_tmp(&self) -> Result<gio::File, Box<dyn Error>> {
    let tmp = gio::File::for_path(TEMP_FOLDER.to_str().unwrap());
    if file_exists(&tmp).await.is_ok_and(|v| !v) {
      log::debug!("create_dir({:?})", &tmp);
      tmp.make_directory_future(glib::Priority::default()).await?;
    }
    let tmp = tmp.child(&self.filename);
    log::debug!("write_to_tmp({:?})", &tmp);
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
