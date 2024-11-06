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
use std::{fmt, fs};

use super::message::TEMP_FOLDER;

#[derive(Debug, Clone)]
pub struct Attachment {
  pub filename: String,
  pub content_id: String,
  pub body: Vec<u8>,
  pub mime_type: Option<String>,
}

impl Attachment {
  pub fn write_to_tmp(&self) -> Result<String, Box<dyn Error>> {
    let mut tmp = TEMP_FOLDER.clone();
    if tmp.exists() == false {
      log::debug!("create_dir({:?})", &tmp);
      fs::create_dir(&tmp)?;
    }
    tmp.push(&self.filename);
    log::debug!("write_to_tmp({:?})", &tmp);
    self.write_to_file(tmp.to_str().unwrap())?;
    Ok(tmp.to_string_lossy().to_string())
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
