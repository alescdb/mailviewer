use adw::prelude::{AdwDialogExt, PreferencesGroupExt};
use gtk4::glib;
use gtk4::prelude::*;
use mailviewer_core::message::headers::Header;

pub fn new(headers: &[Header]) -> adw::Dialog {
  let builder = gtk4::Builder::from_string(gtk4::include_blueprint!("src/headers/headersview.blp"));
  let dialog: adw::Dialog = builder.object("headers_dialog").unwrap();
  let group: adw::PreferencesGroup = builder.object("headers_group").unwrap();

  for header in headers {
    let name = glib::markup_escape_text(&header.name);
    let value = glib::markup_escape_text(&header.value);
    let row = adw::ActionRow::builder()
      .title(name.as_str())
      .subtitle(value.as_str())
      .subtitle_selectable(true)
      .build();
    group.add(&row);
  }

  dialog
}

pub fn show<P: IsA<gtk4::Widget>>(parent: &P, headers: &[Header]) {
  new(headers).present(Some(parent));
}
