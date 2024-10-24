/* main.rs
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

mod application;
mod config;
mod html;
mod mailparser;
mod window;

use self::{application::MailViewerApplication, window::MailViewerWindow};

use config::{APP_ID, PKGDATADIR};
use gtk4::{gio, glib, prelude::*};

fn main() -> glib::ExitCode {
  env_logger::init();

  let resources = gio::Resource::load(PKGDATADIR.to_owned() + "/mailviewer.gresource")
    .expect("Could not load resources");
  gio::resources_register(&resources);

  let app = MailViewerApplication::new(APP_ID, &gio::ApplicationFlags::HANDLES_OPEN);

  app.run()
}
