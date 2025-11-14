/* mailservice.rs
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
use std::cell::RefCell;

use crate::config::VERSION;
use crate::gio::prelude::*;
use crate::message::attachment::Attachment;
use crate::message::message::{Message, MessageParser};
use crate::{gio, glib};

pub struct MailService {
  parser: RefCell<Option<MessageParser>>,
  file: RefCell<Option<gio::File>>,
  show_file_name: RefCell<bool>,
  signal_title_changed: RefCell<Option<Box<dyn Fn(&Self, &str) + 'static>>>,
}

impl MailService {
  pub fn new() -> Self {
    Self {
      parser: RefCell::new(None),
      file: RefCell::new(None),
      show_file_name: RefCell::new(true),
      signal_title_changed: RefCell::new(None),
    }
  }

  pub async fn open_message(
    &self,
    file: &gio::File,
    cancellable: Option<&gio::Cancellable>,
  ) -> Result<(), Box<dyn std::error::Error>> {
    self.file.borrow_mut().replace(file.clone());
    let mut parser = MessageParser::new(file, cancellable).await?;

    let parse_thread = {
      let cancellable = cancellable.cloned().unwrap_or(gio::Cancellable::new());
      gio::spawn_blocking(move || -> Result<MessageParser, glib::Error> {
        let ret = match parser.parse(Some(&cancellable)) {
          Ok(_) => Ok(parser),
          Err(e) => Err(glib::Error::new(gio::IOErrorEnum::Failed, &format!("{e}"))),
        };
        // XXX: Ideally we should cancel the parsing thread earlier, but this is
        // not supported by the API, and it's not worth to rely on GTask API
        // directly to do it.
        cancellable.set_error_if_cancelled()?;
        ret
      })
      .await
      .unwrap()
    };

    match parse_thread {
      Ok(parser) => self.parser.borrow_mut().replace(parser),
      Err(e) => return Err(Box::new(e)),
    };

    self.update_title();
    Ok(())
  }

  pub fn from(&self) -> String {
    if let Some(parser) = self.parser.borrow().as_ref() {
      return parser.from();
    }
    String::new()
  }

  pub fn to(&self) -> String {
    if let Some(parser) = self.parser.borrow().as_ref() {
      return parser.to();
    }
    String::new()
  }

  pub fn subject(&self) -> String {
    if let Some(parser) = self.parser.borrow().as_ref() {
      return parser.subject();
    }
    String::new()
  }

  pub fn date(&self) -> String {
    if let Some(parser) = self.parser.borrow().as_ref() {
      return parser.date();
    }
    String::new()
  }

  pub fn body_text(&self) -> Option<String> {
    if let Some(parser) = self.parser.borrow().as_ref() {
      return parser.body_text();
    }
    None
  }

  pub fn body_html(&self) -> Option<String> {
    if let Some(parser) = self.parser.borrow().as_ref() {
      return parser.body_html();
    }
    None
  }

  pub fn attachments(&self) -> Vec<Attachment> {
    if let Some(parser) = self.parser.borrow().as_ref() {
      return parser.attachments().clone();
    }
    vec![]
  }

  pub fn set_show_file_name(&self, show_file_name: bool) {
    log::debug!("set_show_file_name({})", show_file_name);
    self.show_file_name.replace(show_file_name);
    self.update_title();
  }

  pub fn get_file(&self) -> Option<gio::File> {
    self.file.borrow().clone()
  }

  pub fn connect_title_changed<F: Fn(&Self, &str) + 'static>(&self, f: F) {
    self.signal_title_changed.borrow_mut().replace(Box::new(f));
  }

  fn update_title(&self) {
    if let Some(callback) = self.signal_title_changed.borrow().as_ref() {
      if let Some(file) = self.file.borrow().as_ref() {
        let title = self.get_title(file);
        callback(self, &title);
      }
    }
  }

  fn get_title(&self, file: &gio::File) -> String {
    if *self.show_file_name.borrow() {
      if let Some(filename) = file.basename() {
        return filename.to_string_lossy().to_string();
      }
    }
    format!("Mail Viewer v{}", VERSION)
  }
}

impl std::fmt::Debug for MailService {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("MailService")
      .field("fullpath", &self.file)
      .field("show_file_name", &self.show_file_name)
      .finish()
  }
}

#[cfg(test)]
mod tests {
  use std::rc::Rc;

  use crate::gio::prelude::*;
  use crate::mailservice::MailService;
  use crate::test_utils;
  use crate::{gio, glib};

  #[test]
  fn new_mail_service() {
    let service = MailService::new();

    assert!(service.parser.borrow().is_none());
    assert!(service.file.borrow().is_none());
    assert_eq!(*service.show_file_name.borrow(), true);
  }

  #[test]
  fn open_mail_success() {
    let mail_service = MailService::new();
    let service = mail_service;
    let fullpath = "sample.eml";
    let file = gio::File::for_path(fullpath);

    test_utils::spawn_and_wait(async move {
      assert!(service.open_message(&file, None).await.is_ok());
      assert!(service.get_file().unwrap().equal(&file));
      assert_eq!(service.from(), "John Doe <john@moon.space>");
      assert_eq!(service.to(), "Lucas <lucas@mercure.space>");
      assert_eq!(service.subject(), "Lorem ipsum");
      assert_eq!(service.date(), "2024-10-23 12:27:21");
    });
  }

  #[test]
  fn open_mail_file_not_found() {
    let service = MailService::new();
    let file = gio::File::for_path("path/to/nonexistent.eml");

    test_utils::spawn_and_wait(async move {
      let result = service.open_message(&file, None).await;

      assert!(result.is_err());
      let err = result.unwrap_err();

      if let Some(glib_err) = err.downcast_ref::<glib::Error>() {
        assert!(glib_err.is::<gio::IOErrorEnum>());
        assert!(glib_err.matches(gio::IOErrorEnum::NotFound));
      } else {
        panic!("Expected glib::Error, got: {}", err);
      }
    });
  }

  #[test]
  fn get_text() {
    let service = MailService::new();
    let file = gio::File::for_path("sample.eml");

    test_utils::spawn_and_wait(async move {
      service.open_message(&file, None).await.unwrap();
      let text = service.body_text().unwrap();

      assert!(text.contains("Lorem ipsum dolor sit amet, consectetur adipiscing elit"));
    });
  }

  #[test]
  fn get_html() {
    let service = MailService::new();
    let file = gio::File::for_path("sample.eml");

    test_utils::spawn_and_wait(async move {
      service
        .open_message(&file, Some(&gio::Cancellable::new()))
        .await
        .unwrap();
      let html = service.body_html().unwrap();

      assert!(html.contains("Hello Lucas,"));
    });
  }

  #[test]
  fn get_attachments() {
    let service = MailService::new();
    let file = gio::File::for_path("sample.eml");

    test_utils::spawn_and_wait(async move {
      service.open_message(&file, None).await.unwrap();
      let attachments = service.attachments();

      assert_eq!(attachments.len(), 1);
      assert_eq!(attachments[0].filename, "Deus_Gnome.png");
    });
  }

  #[test]
  fn update_title_with_show_file_name() {
    let service = MailService::new();
    let file = gio::File::for_path("sample.eml");

    test_utils::spawn_and_wait(async move {
      service.open_message(&file, None).await.unwrap();
      service.set_show_file_name(true);

      let file_title = gio::File::for_path("sample_title.eml");
      assert_eq!(service.get_title(&file_title), "sample_title.eml");
    });
  }

  #[test]
  fn update_title_without_show_file_name() {
    let service = MailService::new();
    service.set_show_file_name(false);
    assert_eq!(
      service.get_title(&gio::File::for_path("sample.eml")),
      format!("Mail Viewer v{}", crate::config::VERSION)
    );
  }

  #[test]
  fn connect_title_changed() {
    let service = MailService::new();
    let title_changed_called = Rc::new(std::cell::RefCell::new(false));
    let title_changed_called_clone = Rc::clone(&title_changed_called);
    service.connect_title_changed(move |_, _| {
      *title_changed_called_clone.borrow_mut() = true;
    });

    test_utils::spawn_and_wait(async move {
      let file = gio::File::for_path("sample.eml");
      service.open_message(&file, None).await.unwrap();
      service.set_show_file_name(false);
      assert!(*title_changed_called.borrow());
    });
  }

  #[test]
  fn cancelled_loading() {
    let service = MailService::new();
    let file = gio::File::for_path("sample.eml");

    test_utils::spawn_and_wait(async move {
      let cancellable = gio::Cancellable::new();
      cancellable.cancel();
      let result = service.open_message(&file, Some(&cancellable)).await;
      assert!(result.is_err());

      let err = result.unwrap_err();
      let glib_err = err.downcast_ref::<glib::Error>().unwrap();
      assert!(glib_err.is::<gio::IOErrorEnum>());
      assert!(glib_err.matches(gio::IOErrorEnum::Cancelled));
    });
  }
}
