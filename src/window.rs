/* window.rs
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
use std::option::Option;

use adw::glib::clone;
use adw::prelude::{AlertDialogExt, *};
use adw::subclass::prelude::*;
use gettextrs::{gettext, ngettext};
use gtk4::{gio, glib, template_callbacks};

use crate::mailservice::MailService;
use crate::message::attachment::Attachment;
use crate::message::message::MessageParser;
use crate::utils;
use crate::viewer::capabilities::Capabilities;
use crate::viewer::content::{RenderOptions, ViewerContent};
use crate::viewer::printable::{Printable, PrintableContent};
use crate::viewer::viewer::Viewer;
use crate::viewer::zoomable::Zoomable;

const SETTINGS_SHOW_FILE_NAME: &str = "show-file-name";
const SETTINGS_FORCE_CSS: &str = "force-css";

/// Links in a message are opened by the system handler, so only hand over the
/// schemes a mail is expected to link to.
const ALLOWED_URI_SCHEMES: [&str; 3] = ["http", "https", "mailto"];

const ZOOM_STEP: f64 = 0.1;
const ZOOM_MIN: f64 = 0.3;
const ZOOM_MAX: f64 = 5.0;

mod imp {
  use std::cell::{OnceCell, RefCell};

  use adw::subclass::prelude::CompositeTemplateClass;
  use gtk4::ScrolledWindow;

  use super::*;

  #[derive(Debug, gtk4::CompositeTemplate)]
  #[template(file = "src/window.blp")]
  pub struct MailViewerWindow {
    #[template_child]
    pub from: TemplateChild<gtk4::Entry>,
    #[template_child]
    pub to: TemplateChild<gtk4::Entry>,
    #[template_child]
    pub subject: TemplateChild<gtk4::Entry>,
    #[template_child]
    pub date: TemplateChild<gtk4::Entry>,
    #[template_child]
    pub viewer: TemplateChild<Viewer>,
    #[template_child]
    pub force_css: TemplateChild<gtk4::ToggleButton>,
    #[template_child]
    pub zoom_minus: TemplateChild<gtk4::Button>,
    #[template_child]
    pub zoom_plus: TemplateChild<gtk4::Button>,
    #[template_child]
    pub show_images: TemplateChild<gtk4::ToggleButton>,
    #[template_child]
    pub show_text: TemplateChild<gtk4::ToggleButton>,
    #[template_child]
    pub pull_label: TemplateChild<gtk4::Label>,
    #[template_child]
    pub sheet: TemplateChild<adw::BottomSheet>,
    #[template_child]
    pub content_box: TemplateChild<gtk4::Box>,
    #[template_child]
    pub attachments_clamp: TemplateChild<adw::Clamp>,
    #[template_child]
    pub search_bar: TemplateChild<gtk4::SearchBar>,
    #[template_child]
    pub search_entry: TemplateChild<gtk4::SearchEntry>,
    //
    pub scrolled_window: ScrolledWindow,
    pub settings: OnceCell<gio::Settings>,
    pub service: MailService,
    pub cancellable: RefCell<gio::Cancellable>,
  }

  impl Default for MailViewerWindow {
    fn default() -> Self {
      // Nothing a message pulls in has any business surviving on disk, so keep
      // the cache and the cookies of the session in memory only.
      MailViewerWindow {
        scrolled_window: ScrolledWindow::new(),
        from: TemplateChild::default(),
        to: TemplateChild::default(),
        subject: TemplateChild::default(),
        date: TemplateChild::default(),
        viewer: TemplateChild::default(),
        show_images: TemplateChild::default(),
        force_css: TemplateChild::default(),
        zoom_minus: TemplateChild::default(),
        zoom_plus: TemplateChild::default(),
        show_text: TemplateChild::default(),
        pull_label: TemplateChild::default(),
        attachments_clamp: TemplateChild::default(),
        search_bar: TemplateChild::default(),
        search_entry: TemplateChild::default(),
        content_box: TemplateChild::default(),
        sheet: TemplateChild::default(),
        settings: OnceCell::new(),
        service: MailService::new(),
        cancellable: RefCell::new(gio::Cancellable::new()),
      }
    }
  }

  #[glib::object_subclass]
  impl ObjectSubclass for MailViewerWindow {
    type Interfaces = ();
    type ParentType = adw::ApplicationWindow;
    type Type = super::MailViewerWindow;

    const ABSTRACT: bool = false;
    const NAME: &'static str = "MailViewerWindow";

    fn class_init(klass: &mut Self::Class) {
      Viewer::static_type();
      klass.bind_template();
      klass.bind_template_instance_callbacks();
      klass.install_action_async(
        "win.open-file-dialog",
        None,
        |window, _, parameter: Option<glib::Variant>| async move {
          let mut close = false;
          if let Some(param) = parameter {
            close = param.get::<bool>().unwrap_or(false);
          }
          window.open_file_dialog(close).await;
        },
      );
      klass.install_action_async("win.print", None, |window, _, _| async move {
        window.print().await;
      });
      klass.install_action_async(
        "win.open-file",
        None,
        |window, _, parameter: Option<glib::Variant>| async move {
          let mut filename: Option<String> = None;
          if let Some(parameter) = parameter {
            filename = parameter.get::<Option<String>>().unwrap();
          }
          if let Some(filename) = filename {
            let file = if filename.starts_with("/") {
              gio::File::for_path(filename.as_str())
            } else {
              gio::File::for_uri(filename.as_str())
            };
            window.open_file(&file).await;
          } else {
            window.open_file_dialog(true).await;
          }
        },
      );
      klass.install_action("win.search", None, move |win, _, _| {
        win.start_search();
      });
      klass.install_action("win.preferences", None, move |win, _, _| {
        win.show_preferences();
      });
      klass.install_action("win.reset-zoom", None, move |win, _, _| {
        win.reset_zoom();
      });
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
      obj.init_template();
    }
  }

  impl ObjectImpl for MailViewerWindow {}
  impl WidgetImpl for MailViewerWindow {}
  impl WindowImpl for MailViewerWindow {}
  impl ApplicationWindowImpl for MailViewerWindow {}
  impl AdwApplicationWindowImpl for MailViewerWindow {}
}

glib::wrapper! {
    pub struct MailViewerWindow(ObjectSubclass<imp::MailViewerWindow>)
        @extends
            gtk4::Widget,
            gtk4::Window,
            gtk4::ApplicationWindow,
            adw::ApplicationWindow,
        @implements
            gtk4::Buildable,
            gtk4::ConstraintTarget,
            gtk4::Accessible,
            gtk4::Native,
            gtk4::Root,
            gtk4::ShortcutManager,
            gio::ActionGroup,
            gio::ActionMap;
}

#[template_callbacks]
impl MailViewerWindow {
  pub fn new<P: IsA<gtk4::Application>>(application: &P) -> Self {
    let window: Self = glib::Object::builder()
      .property("application", application)
      .build();

    window.initialize();
    window
  }

  #[template_callback]
  pub fn on_force_css_clicked(&self) {
    log::debug!("on_force_css_clicked()");
    self.update_render_options();
  }

  #[template_callback]
  pub fn on_show_text_clicked(&self) {
    let show = self.imp().show_text.is_active();
    log::debug!("on_show_text_clicked({})", show);
    self.on_show_text(show);
  }

  #[template_callback]
  pub fn on_show_images_clicked(&self) {
    let show = self.imp().show_images.is_active();
    log::debug!("on_show_images_clicked({})", show);
    self.update_render_options();
  }

  fn start_search(&self) {
    if !self.imp().viewer.has_search() {
      return;
    }
    self.imp().search_bar.set_search_mode(true);
    self.imp().search_entry.grab_focus();
  }

  #[template_callback]
  pub fn on_search_changed(&self) {
    let text = self.imp().search_entry.text();
    log::debug!("on_search_changed({})", text);
    self.imp().viewer.find(&text);
  }

  #[template_callback]
  pub fn on_search_next(&self) {
    self.imp().viewer.find_next();
  }

  #[template_callback]
  pub fn on_search_previous(&self) {
    self.imp().viewer.find_previous();
  }

  #[template_callback]
  pub fn on_search_stopped(&self) {
    log::debug!("on_search_stopped()");
    self.imp().viewer.clear_find();
    self.imp().search_bar.set_search_mode(false);
  }

  #[template_callback]
  pub fn on_zoom_minus_clicked(&self) {
    log::debug!("on_zoom_minus_clicked()");
    self.set_zoom_level(self.zoom_level() - ZOOM_STEP);
  }

  #[template_callback]
  pub fn on_zoom_plus_clicked(&self) {
    log::debug!("on_zoom_plus_clicked()");
    self.set_zoom_level(self.zoom_level() + ZOOM_STEP);
  }

  fn zoom_level(&self) -> f64 {
    self.imp().viewer.zoom()
  }

  fn initialize(&self) {
    log::debug!("initialize()");

    self.initialize_settings();
    self.initialize_actions();
    self.update_capabilities();
  }

  fn update_capabilities(&self) {
    let viewer = &self.imp().viewer;
    self.imp().show_images.set_visible(viewer.has_images());
    self.imp().force_css.set_visible(viewer.has_html());
    self.imp().search_bar.set_visible(viewer.has_search());
    self.imp().zoom_minus.set_visible(viewer.has_zoom());
    self.imp().zoom_plus.set_visible(viewer.has_zoom());
    self.action_set_enabled("print", viewer.has_print());
  }

  fn render_options(&self) -> RenderOptions {
    RenderOptions {
      force_css: self.imp().force_css.is_active(),
      allow_remote: self.imp().show_images.is_active(),
      dark: adw::StyleManager::default().is_dark(),
    }
  }

  fn update_render_options(&self) {
    self.imp().viewer.set_render_options(self.render_options());
  }

  fn initialize_actions(&self) {
    let win = self;
    let imp = self.imp();

    let drop_target = gtk4::DropTarget::new(gio::File::static_type(), gtk4::gdk::DragAction::COPY);
    imp.viewer.add_controller(drop_target.clone());
    drop_target.connect_drop(clone!(
      #[strong]
      win,
      move |_, data, _, _| {
        if let Ok(file) = data.get::<gio::File>() {
          glib::spawn_future_local(glib::clone!(
            #[strong]
            win,
            #[weak]
            file,
            async move {
              win.open_file(&file).await;
            }
          ));
        }

        false
      }
    ));

    adw::StyleManager::default().connect_dark_notify(clone!(
      #[weak(rename_to = window)]
      self,
      move |_| {
        if window.imp().force_css.is_active() {
          log::debug!("colour scheme changed, rendering again");
          window.update_render_options();
        }
      }
    ));

    imp.viewer.connect_link_handler(clone!(
      #[strong]
      win,
      move |uri| win.open_uri(uri)
    ));
  }

  fn initialize_settings(&self) {
    let settings = gio::Settings::new(crate::config::APP_ID);
    let imp = self.imp();

    imp.settings.set(settings.clone()).unwrap();
    let zoom = settings.get::<f64>("zoom").clamp(ZOOM_MIN, ZOOM_MAX);
    imp.viewer.set_zoom(zoom);

    settings
      .bind("width", self, "default-width")
      .flags(gio::SettingsBindFlags::DEFAULT)
      .build();
    settings
      .bind("height", self, "default-height")
      .flags(gio::SettingsBindFlags::DEFAULT)
      .build();
    settings
      .bind("is-maximized", self, "maximized")
      .flags(gio::SettingsBindFlags::DEFAULT)
      .build();
    settings
      .bind("is-fullscreen", self, "fullscreened")
      .flags(gio::SettingsBindFlags::DEFAULT)
      .build();

    imp.service.connect_title_changed(clone!(
      #[weak(rename_to = window)]
      self,
      move |_, title| {
        window.set_title(Some(title));
      }
    ));

    imp
      .service
      .set_show_file_name(self.get_settings_show_file_name());
    imp
      .force_css
      .set_active(self.get_settings_bool(SETTINGS_FORCE_CSS));
  }

  fn reset_zoom(&self) {
    log::debug!("reset_zoom()");
    self.set_zoom_level(1.0);
  }

  fn add_attachment(&self, attachment: &Attachment, preferences_group: &adw::PreferencesGroup) {
    let window = self;
    let mime = &attachment
      .clone()
      .mime_type
      .clone()
      .unwrap_or("Unknown".to_string());
    let icon = if mime.starts_with("image") {
      "image-x-generic-symbolic"
    } else {
      "document-open"
    };

    let save = gtk4::Button::new();
    save.set_valign(gtk4::Align::Center);
    save.set_icon_name("document-save-as-symbolic");
    save.set_tooltip_text(Some(&gettext("Save as...")));
    save.connect_clicked(clone!(
      #[strong]
      window,
      #[strong]
      attachment,
      move |_| {
        glib::spawn_future_local(glib::clone!(
          #[strong]
          window,
          #[strong]
          attachment,
          async move {
            window.on_attachment_save(&attachment).await;
          }
        ));
      }
    ));
    // The file name and the mime type come from the message, don't let them
    // through as pango markup.
    let btn = adw::ActionRow::builder()
      .title(attachment.filename.to_string())
      .subtitle(mime)
      .use_markup(false)
      .activatable(true)
      .build();
    btn.add_prefix(&gtk4::Image::from_icon_name(icon));
    btn.add_suffix(&save);

    btn.connect_activated(clone!(
      #[strong]
      window,
      #[strong]
      btn,
      #[strong]
      attachment,
      move |_| {
        glib::spawn_future_local(glib::clone!(
          #[strong]
          window,
          #[strong]
          attachment,
          #[strong]
          btn,
          async move {
            btn.set_sensitive(false);
            window.on_attachment_open(&attachment).await;
            btn.set_sensitive(true);
          }
        ));
      }
    ));
    preferences_group.add(&btn);
  }

  async fn on_attachment_save(&self, attachment: &Attachment) {
    log::debug!("on_attachment_save({})", attachment.filename);

    let current_file = self.imp().service.get_file().unwrap();
    let initial_file = current_file
      .parent()
      .unwrap()
      .child(attachment.safe_filename());

    let save_dialog = gtk4::FileDialog::builder()
      .title(gettext("Save attachment..."))
      .modal(true)
      .initial_file(&initial_file)
      .build();

    match save_dialog.save_future(Some(self)).await {
      Ok(file) => {
        let path = file.peek_path().unwrap_or_default();
        let path = path.display();
        log::debug!("Saving attachment to {:?}", path);
        match attachment.write_to_file(&file).await {
          Ok(_) => log::debug!("write_to_file({:?})", path),
          Err(e) => {
            log::error!("write_to_file({})", e);
            self.alert_error(&gettext("File Error"), &e.to_string(), false);
          }
        };
      }
      Err(e) => match e.kind() {
        Some(gtk4::DialogError::Dismissed) | Some(gtk4::DialogError::Cancelled) => (),
        _ => log::error!("save_dialog({})", e),
      },
    }
  }

  async fn on_attachment_open(&self, attachment: &Attachment) {
    log::debug!("on_button_clicked({})", attachment.filename);
    match attachment.write_to_tmp().await {
      Ok(file) => {
        let path = file.peek_path().unwrap().to_string_lossy().to_string();
        log::debug!("write_to_tmp({}) success", path);

        if let Err(e) = gtk4::FileLauncher::new(Some(&file))
          .launch_future(Some(self))
          .await
        {
          log::error!("{} ({}): {}", gettext("Failed to open file"), path, e);
        }
      }
      Err(e) => log::error!("write_to_tmp({})", e),
    };
  }

  fn set_zoom_level(&self, zoom: f64) {
    let zoom = zoom.clamp(ZOOM_MIN, ZOOM_MAX);
    log::debug!("set_zoom({})", zoom);
    self.imp().viewer.set_zoom(zoom);

    if zoom <= ZOOM_MIN {
      self.imp().zoom_minus.set_sensitive(false);
    } else {
      self.imp().zoom_minus.set_sensitive(true);
    }
    if zoom >= ZOOM_MAX {
      self.imp().zoom_plus.set_sensitive(false);
    } else {
      self.imp().zoom_plus.set_sensitive(true);
    }

    if let Some(settings) = self.imp().settings.get() {
      let _ = settings.set("zoom", zoom);
    }
  }

  fn open_uri(&self, uri: String) {
    let allowed = utils::uri_scheme(&uri)
      .is_some_and(|scheme| ALLOWED_URI_SCHEMES.contains(&scheme.as_str()));
    if !allowed {
      log::warn!("URI refused => {}", uri);
      return;
    }

    log::debug!("URI launch => {}", uri);
    glib::spawn_future_local(glib::clone!(
      #[strong(rename_to = window)]
      self,
      async move {
        if let Err(e) = gtk4::UriLauncher::new(&uri).launch_future(Some(&window)).await {
          log::error!("{} ({}): {}", gettext("Failed to open uri"), uri, e);
        }
      }
    ));
  }

  fn on_show_text(&self, show: bool) {
    log::debug!("on_show_text({})", show);
    let imp = self.imp();

    imp.viewer.set_text_visible(show);

    imp.show_text.set_active(show);
    self.update_capabilities();
  }

  fn build_mail_file_dialog(&self, title: &String) -> gtk4::FileDialog {
    let filter = gtk4::FileFilter::new();
    filter.set_name(Some(&gettext("Mail Files")));
    filter.add_pattern("*.eml");
    filter.add_pattern("*.msg");

    for mime in MessageParser::supported_mime_types() {
      filter.add_mime_type(mime);
    }

    let filters = gio::ListStore::new::<gtk4::FileFilter>();
    filters.append(&filter);
    gtk4::FileDialog::builder()
      .title(title)
      .modal(true)
      .filters(&filters)
      .build()
  }

  fn printable_content(&self) -> PrintableContent {
    let imp = self.imp();
    PrintableContent {
      html: imp.service.body_html(),
      text: imp.service.body_text(),
      from: imp.service.from(),
      to: imp.service.to(),
      date: imp.service.date(),
      subject: imp.service.subject(),
      attachments: imp.service.attachments(),
    }
  }

  pub async fn print(&self) {
    log::debug!("print()");

    if !self.imp().viewer.has_print() {
      return;
    }

    let content = self.printable_content();
    let parent: &gtk4::Window = self.upcast_ref();
    self
      .imp()
      .viewer
      .print(parent, &content, self.render_options());
  }

  pub async fn open_file_dialog(&self, close_on_cancel: bool) -> bool {
    log::debug!("open_file_dialog()");

    let load_dialog = self.build_mail_file_dialog(&gettext("Open Mail File"));
    match load_dialog.open_future(Some(self)).await {
      Ok(file) => {
        self.open_file(&file).await;
        return true;
      }
      Err(e) => match e.kind() {
        Some(gtk4::DialogError::Dismissed) | Some(gtk4::DialogError::Cancelled) => {
          if close_on_cancel {
            self.close();
          }
        }
        _ => log::error!("open_file_dialog({})", e),
      },
    }

    false
  }

  pub async fn open_file(&self, file: &gio::File) {
    log::debug!("open_file({:?})", file.peek_path().unwrap_or_default());

    self.on_show_text(true);
    self.on_search_stopped();
    self.imp().content_box.get().set_sensitive(false);
    self.imp().sheet.get().set_open(false);

    let cancellable = gio::Cancellable::new();
    {
      self.imp().cancellable.replace_with(|old_cancellable| {
        old_cancellable.cancel();
        cancellable.clone()
      });
    }

    match self
      .imp()
      .service
      .open_message(file, Some(&cancellable))
      .await
    {
      Ok(_) => {
        self.display_message();
      }
      Err(e) => {
        if cancellable.is_cancelled() {
          log::debug!(
            "Ignoring loading of {}, action was cancelled",
            file.peek_path().unwrap_or_default().display()
          );
          return;
        }
        log::error!("service(ERR) : {}", e);
        self.alert_error(
          &gettext("File Error"),
          &format!("{}:\n{}", gettext("Failed to open file"), e),
          true,
        );
      }
    };
    self.imp().content_box.get().set_sensitive(true);
  }

  pub fn display_message(&self) {
    log::debug!("display_eml()");
    let imp = self.imp();

    imp.from.set_text(imp.service.from().as_str());
    imp.date.set_text(imp.service.date().as_str());
    imp.to.set_text(imp.service.to().as_str());
    imp.subject.set_text(imp.service.subject().as_str());

    let content = ViewerContent {
      html: imp.service.body_html(),
      text: imp.service.body_text(),
      attachments: imp.service.attachments(),
    };
    let has_text = content.text.is_some();
    let has_html = content.html.is_some();

    imp.viewer.load_content(content, self.render_options());

    imp.show_text.set_visible(has_text && has_html);
    self.on_show_text(!has_html);

    let preferences_group: adw::PreferencesGroup = adw::PreferencesGroup::new();
    self
      .imp()
      .attachments_clamp
      .set_child(Some(&preferences_group));

    let attachments = imp.service.attachments();
    let total = attachments.len();
    if total > 0 {
      for attachment in &attachments {
        self.add_attachment(attachment, &preferences_group);
      }
      let fmt: String = ngettext(
        "{total} attachment",
        "{total} attachments",
        total.try_into().unwrap(),
      )
      .replace("{total}", &total.to_string());
      log::debug!("display_message() => {}", fmt);
      preferences_group.set_title(&fmt);
      imp.pull_label.set_text(&fmt);
    } else {
      // never shown
      imp.pull_label.set_text(&gettext("No attachments"));
    }

    if let Some(widget) = imp.sheet.bottom_bar() {
      if total > 0 {
        widget.set_visible(true)
      } else {
        widget.set_visible(false)
      }
    }
  }

  pub fn alert_error(&self, title: &str, message: &str, close_window: bool) -> adw::AlertDialog {
    let alert = adw::AlertDialog::new(Some(title), Some(message));
    alert.add_response("close", &gettext("Close"));
    alert.set_response_appearance("close", adw::ResponseAppearance::Destructive);
    alert.present(Some(self));
    if close_window {
      alert.connect_response(
        Some("close"),
        clone!(
          #[strong(rename_to = window)]
          self,
          move |_, _| {
            window.close();
          }
        ),
      );
    }
    alert
  }

  fn set_force_css(&self, force: bool) {
    log::debug!("set_force_css({})", force);
    self.imp().force_css.set_active(force);
    self.update_render_options();
  }

  fn get_settings_bool(&self, key: &str) -> bool {
    if let Some(settings) = self.imp().settings.get() {
      settings.get::<bool>(key)
    } else {
      false
    }
  }

  fn get_settings_show_file_name(&self) -> bool {
    self.get_settings_bool(SETTINGS_SHOW_FILE_NAME)
  }

  fn get_settings_force_css(&self) -> bool {
    self.get_settings_bool(SETTINGS_FORCE_CSS)
  }

  fn show_preferences(&self) {
    log::debug!("show_preferences()");
    match self.imp().settings.get() {
      Some(settings) => {
        let builder = gtk4::Builder::from_string(gtk4::include_blueprint!("src/preferences.blp"));
        let show_file_name: adw::SwitchRow = builder.object("show_file_name").unwrap();
        let force_css: adw::SwitchRow = builder.object("force_css").unwrap();
        settings
          .bind(SETTINGS_SHOW_FILE_NAME, &show_file_name, "active")
          .build();
        settings
          .bind(SETTINGS_FORCE_CSS, &force_css, "active")
          .build();

        let prefs: adw::PreferencesDialog = builder.object("preferences").unwrap();
        prefs.present(Some(self));
        prefs.connect_closed(clone!(
          #[weak(rename_to = win)]
          self,
          move |_| {
            log::debug!("show_preferences() => done");
            win
              .imp()
              .service
              .set_show_file_name(win.get_settings_show_file_name());
            win.set_force_css(win.get_settings_force_css());
          }
        ));
      }
      None => {
        self.alert_error(
          &gettext("Settings"),
          &gettext("Failed to get settings"),
          false,
        );
      }
    }
  }
}
