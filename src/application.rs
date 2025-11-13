/* application.rs
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
use adw::prelude::AdwDialogExt;
use adw::subclass::prelude::*;
use gtk4::prelude::*;
use gtk4::{gio, glib};

use crate::config::{APP_ID, VERSION};
use crate::MailViewerWindow;

mod imp {
  use std::cell::RefCell;

  use super::*;

  #[derive(Debug, Default)]
  pub struct MailViewerApplication {
    filename: RefCell<Option<String>>,
  }

  #[glib::object_subclass]
  impl ObjectSubclass for MailViewerApplication {
    type Interfaces = ();
    type ParentType = adw::Application;
    type Type = super::MailViewerApplication;

    const NAME: &'static str = "MailViewerApplication";
  }

  impl ObjectImpl for MailViewerApplication {
    fn constructed(&self) {
      self.parent_constructed();
      let obj = self.obj();
      obj.setup_gactions();
      obj.set_accels_for_action("app.quit", &["<primary>q"]);
      obj.set_accels_for_action("win.open-file-dialog", &["<primary>o"]);
      obj.set_accels_for_action("win.reset-zoom", &["<primary>r"]);
    }
  }

  impl ApplicationImpl for MailViewerApplication {
    fn activate(&self) {
      let application = self.obj();
      let window: MailViewerWindow = if let Some(window) = application.active_window() {
        window.downcast::<MailViewerWindow>().ok().unwrap()
      } else {
        let window = MailViewerWindow::new(&*application);
        let provider = gtk4::CssProvider::new();

        provider.load_from_resource("/io/github/alescdb/mailviewer/css/style.css");
        if let Some(display) = gtk4::gdk::Display::default() {
          gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
          );
        }
        window.upcast()
      };
      window.present();
      if let Err(e) = adw::prelude::WidgetExt::activate_action(
        &window,
        "win.open-file",
        Some(&glib::Variant::from(self.filename.borrow().clone())),
      ) {
        log::debug!("open_file_dialog({e})");
        window.alert_error("File Error", &e.to_string(), false);
      }
    }

    fn open(&self, files: &[gio::File], hint: &str) {
      for file in files {
        log::debug!("[ARGUMENT] File: {:?}, Hint : {:?}", file.path(), hint);
      }

      if files.is_empty() == false {
        if let Some(path) = files[0].path() {
          let path = path.to_str().unwrap().to_string();
          self.filename.replace(Some(path.clone()));
        }
      }
      self.activate();
    }
  }

  impl GtkApplicationImpl for MailViewerApplication {}
  impl AdwApplicationImpl for MailViewerApplication {}
}

glib::wrapper! {
  pub struct MailViewerApplication(ObjectSubclass<imp::MailViewerApplication>)
      @extends gio::Application, gtk4::Application, adw::Application,
      @implements gio::ActionGroup, gio::ActionMap;
}

impl MailViewerApplication {
  pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
    glib::Object::builder()
      .property("application-id", application_id)
      .property("flags", flags)
      .build()
  }

  fn setup_gactions(&self) {
    let quit_action = gio::ActionEntry::builder("quit")
      .activate(move |app: &Self, _, _| app.quit())
      .build();
    let about_action = gio::ActionEntry::builder("about")
      .activate(move |app: &Self, _, _| app.show_about())
      .build();
    self.add_action_entries([quit_action, about_action]);
  }

  fn show_about(&self) {
    let window = self.active_window().unwrap();
    let dialog = adw::AboutDialog::builder()
      .application_icon(APP_ID)
      .application_name("MailViewer")
      .developer_name("Alexandre Del Bigio")
      .version(VERSION)
      .copyright("© 2024 Alexandre Del Bigio")
      .license_type(gtk4::License::Gpl30)
      .developers(vec!["Alexandre Del Bigio", "Marco Trevisan (Treviño)"])
      .issue_url("https://github.com/alescdb/mailviewer/issues")
      .support_url("https://github.com/alescdb")
      .build();

    dialog.add_link("GitHub", "https://github.com/alescdb/mailviewer");
    dialog.present(Some(&window));
  }
}
