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
use crate::{application::MailViewerApplication, html::Html, mailservice::MailService, message::attachment::Attachment};
use adw::{
  glib::clone,
  prelude::{AlertDialogExt, *},
  subclass::prelude::*,
};
use gtk4::{gio, glib, prelude::FileChooserExt, template_callbacks, ResponseType};
use std::{borrow::BorrowMut, option::Option};
use webkit6::{
  prelude::{PolicyDecisionExt, WebViewExt},
  NavigationPolicyDecision, PolicyDecision, PolicyDecisionType, WebView,
};

const SETTINGS_SHOW_FILE_NAME: &str = "show-file-name";

mod imp {
  use super::*;
  use adw::subclass::prelude::CompositeTemplateClass;
  use gtk4::ScrolledWindow;
  use std::cell::OnceCell;

  #[derive(Debug, gtk4::CompositeTemplate)]
  #[template(resource = "/io/github/alescdb/mailviewer/window.ui")]
  pub struct MailViewerWindow {
    #[template_child]
    pub eml_from: TemplateChild<gtk4::Entry>,
    #[template_child]
    pub eml_to: TemplateChild<gtk4::Entry>,
    #[template_child]
    pub eml_subject: TemplateChild<gtk4::Entry>,
    #[template_child]
    pub eml_date: TemplateChild<gtk4::Entry>,
    #[template_child]
    pub placeholder: TemplateChild<gtk4::ScrolledWindow>,
    #[template_child]
    pub force_css: TemplateChild<gtk4::ToggleButton>,
    #[template_child]
    pub zoom_minus: TemplateChild<gtk4::Button>,
    #[template_child]
    pub zoom_plus: TemplateChild<gtk4::Button>,
    #[template_child]
    pub body_text: TemplateChild<gtk4::TextView>,
    #[template_child]
    pub show_images: TemplateChild<gtk4::ToggleButton>,
    #[template_child]
    pub show_text: TemplateChild<gtk4::ToggleButton>,
    #[template_child]
    pub stack: TemplateChild<adw::ViewStack>,
    #[template_child]
    pub pull_label: TemplateChild<gtk4::Label>,
    #[template_child]
    pub sheet: TemplateChild<adw::BottomSheet>,
    #[template_child]
    pub attachments_clamp: TemplateChild<adw::Clamp>,
    //
    pub scrolled_window: ScrolledWindow,
    pub web_view: webkit6::WebView,
    pub web_settings: webkit6::Settings,
    pub settings: OnceCell<gio::Settings>,
    pub service: MailService,
  }

  impl Default for MailViewerWindow {
    fn default() -> Self {
      let window = MailViewerWindow {
        web_view: WebView::new(),
        web_settings: webkit6::Settings::new(),
        scrolled_window: ScrolledWindow::new(),
        eml_from: TemplateChild::default(),
        eml_to: TemplateChild::default(),
        eml_subject: TemplateChild::default(),
        eml_date: TemplateChild::default(),
        placeholder: TemplateChild::default(),
        show_images: TemplateChild::default(),
        force_css: TemplateChild::default(),
        zoom_minus: TemplateChild::default(),
        zoom_plus: TemplateChild::default(),
        show_text: TemplateChild::default(),
        body_text: TemplateChild::default(),
        stack: TemplateChild::default(),
        pull_label: TemplateChild::default(),
        attachments_clamp: TemplateChild::default(),
        sheet: TemplateChild::default(),
        settings: OnceCell::new(),
        service: MailService::new(),
      };
      window
    }
  }

  #[glib::object_subclass]
  impl ObjectSubclass for MailViewerWindow {
    const NAME: &'static str = "MailViewerWindow";
    const ABSTRACT: bool = false;
    type Type = super::MailViewerWindow;
    type ParentType = adw::ApplicationWindow;
    type Interfaces = ();

    fn class_init(klass: &mut Self::Class) {
      klass.bind_template();
      klass.bind_template_instance_callbacks();
      klass.install_action_async(
        "win.open-file",
        None,
        |window, _, parameter: Option<glib::Variant>| async move {
          let mut close = false;
          if let Some(param) = parameter {
            close = param.get::<bool>().unwrap_or(false);
          }
          window
            .open_file_dialog(close)
            .await
            .expect("Error open_file_dialog()");
        },
      );
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
        @extends gtk4::Widget, gtk4::Window, gtk4::ApplicationWindow, adw::ApplicationWindow, @implements gio::ActionGroup, gio::ActionMap;
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
    self.load_html(self.imp().force_css.is_active());
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
    self.imp().web_settings.set_auto_load_images(show);
  }

  #[template_callback]
  pub fn on_zoom_minus_clicked(&self) {
    log::debug!("on_zoom_minus_clicked()");
    self.set_zoom_level(self.imp().web_view.zoom_level() - 0.1);
  }

  #[template_callback]
  pub fn on_zoom_plus_clicked(&self) {
    log::debug!("on_zoom_plus_clicked()");
    self.set_zoom_level(self.imp().web_view.zoom_level() + 0.1);
  }

  fn initialize(&self) {
    log::debug!("initialize()");
    let imp = self.imp();

    self.initialize_settings();
    self.initialize_actions();

    imp.web_settings.set_enable_javascript(false);
    imp.web_settings.set_auto_load_images(false);
    imp.web_view.set_settings(&imp.web_settings);
    imp.placeholder.set_child(Some(&imp.web_view));

    self.open_or_ask();
  }

  fn initialize_actions(&self) {
    let win = self;
    let imp = self.imp();

    imp.web_view.connect_decide_policy(clone!(
      #[strong]
      win,
      move |webview: &WebView, policy: &PolicyDecision, decision_type: PolicyDecisionType| {
        return win.on_decide_policy(webview, policy, decision_type);
      }
    ));
  }

  fn initialize_settings(&self) {
    let settings = gio::Settings::new(crate::config::APP_ID);
    let imp = self.imp();

    imp.settings.set(settings.clone()).unwrap();
    imp.web_view.set_zoom_level(settings.get::<f64>("zoom"));

    settings
      .bind("width", self, "default-width")
      .flags(gio::SettingsBindFlags::DEFAULT)
      .build();
    settings
      .bind("height", self, "default-height")
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
    imp.service.set_show_file_name(self.get_show_file_name());
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
      .unwrap_or("Unknown".to_string());
    let icon = if mime.starts_with("image") {
      "image-x-generic-symbolic"
    } else {
      "document-open"
    };

    let save = gtk4::Button::new();
    save.set_valign(gtk4::Align::Center);
    save.set_icon_name("document-save-as-symbolic");
    save.set_tooltip_text(Some("Save as..."));
    save.connect_clicked(clone!(
      #[strong]
      window,
      #[strong]
      attachment,
      move |_| {
        window.on_attachment_save(&attachment);
      }
    ));
    let btn = adw::ActionRow::builder()
      .title(attachment.filename.to_string())
      .subtitle(mime)
      .activatable(true)
      .build();
    btn.add_prefix(&gtk4::Image::from_icon_name(icon));
    btn.add_suffix(&save);

    btn.connect_activated(clone!(
      #[strong]
      window,
      #[strong]
      attachment,
      move |_| {
        window.on_attachment_open(&attachment);
      }
    ));
    preferences_group.add(&btn);
  }

  fn on_attachment_save(&self, attachment: &Attachment) {
    log::debug!("on_attachment_save({})", attachment.filename);
    let win = self;
    let save_dialog = gtk4::FileChooserDialog::new(
      Some("Save attachment..."),
      Some(self),
      gtk4::FileChooserAction::Save,
      &[
        ("_Cancel", gtk4::ResponseType::Cancel),
        ("_Save", gtk4::ResponseType::Accept),
      ],
    );
    save_dialog.set_modal(true);
    save_dialog.set_current_name(&attachment.filename);
    save_dialog.connect_response(clone!(
      #[strong]
      win,
      #[strong]
      attachment,
      move |dialog, response| {
        if response == gtk4::ResponseType::Accept {
          let path = dialog.file().unwrap().path().unwrap();
          log::debug!("Saving attachment to {:?}", path);
          match attachment.write_to_file(path.to_str().unwrap()) {
            Ok(_) => log::debug!("write_to_file({:?})", &path),
            Err(e) => {
              log::error!("write_to_file({})", e);
              win.alert_error("File Error", &e.to_string());
            }
          };
        }
        dialog.close();
      }
    ));
    save_dialog.show();
  }

  fn on_attachment_open(&self, attachment: &Attachment) {
    log::debug!("on_button_clicked({})", attachment.filename);
    match attachment.write_to_tmp() {
      Ok(file) => {
        log::debug!("write_to_tmp({}) success", &file);
        if let Err(e) = open::that(&file) {
          log::error!("failed to open file ({}): {}", &file, e);
        }
      }
      Err(e) => log::error!("write_to_tmp({})", e),
    };
  }

  fn set_zoom_level(&self, zoom: f64) {
    log::debug!("set_zoom({})", zoom);
    self.imp().web_view.set_zoom_level(zoom);
    let _ = self
      .imp()
      .settings
      .get()
      .expect("Error settings !")
      .set("zoom", zoom);
  }

  fn load_html(&self, force_css: bool) {
    log::debug!("load_html({})", force_css);
    let html = self.imp().service.get_html().unwrap_or(String::new());
    self
      .imp()
      .web_view
      .load_html(&*Html::new(&html, force_css).safe(), None);
  }

  fn on_decide_policy(
    &self,
    _webview: &WebView,
    policy: &PolicyDecision,
    decision_type: PolicyDecisionType,
  ) -> bool {
    if decision_type == PolicyDecisionType::NavigationAction
      || decision_type == PolicyDecisionType::NewWindowAction
    {
      let policy = policy
        .clone()
        .downcast::<NavigationPolicyDecision>()
        .expect("Unable to cast policy");
      let navigation_action = policy.navigation_action();
      if let Some(mut navigation_action) = navigation_action {
        let request = navigation_action
          .borrow_mut()
          .request()
          .expect("Unable to get request");
        let uri = request.uri();
        if let Some(uri) = uri {
          if uri.starts_with("about:") {
            return false;
          }
          log::debug!("WebView on_decide_policy(open) => {}", uri);
          if let Err(e) = open::that(uri.to_string()) {
            self.alert_error("WebView Error", &format!("Failed to open url: {}", e));
          }
        }

        policy.ignore();
        return true;
      }
    }
    false
  }

  fn on_show_text(&self, show: bool) {
    log::debug!("on_show_text({})", show);
    let imp = self.imp();
    imp
      .stack
      .get()
      .set_visible_child_name(if show { "text" } else { "html" });

    imp.show_images.set_visible(!show);
    imp.force_css.set_visible(!show);
    imp.zoom_minus.set_visible(!show);
    imp.zoom_plus.set_visible(!show);
  }

  pub async fn open_file_dialog(
    &self,
    _close_on_cancel: bool,
  ) -> Result<(), Box<dyn std::error::Error>> {
    log::debug!("open_file_dialog()");
    let load_dialog = gtk4::FileChooserDialog::new(
      Some("Open EML File"),
      Some(self),
      gtk4::FileChooserAction::Open,
      &[
        ("_Cancel", gtk4::ResponseType::Cancel),
        ("_Open", gtk4::ResponseType::Accept),
      ],
    );
    let filter = gtk4::FileFilter::new();
    filter.add_pattern("*.eml");
    filter.set_name(Some("EML Files"));
    load_dialog.set_filter(&filter);
    load_dialog.set_modal(true);
    let response: ResponseType = load_dialog.run_future().await;
    log::debug!("open_file_dialog() => {:?}", response);
    if response == gtk4::ResponseType::Accept {
      if let Some(file) = load_dialog.file() {
        if let Some(path) = file.path() {
          self.open_file(path.to_str().unwrap());
        }
      }
    } else if _close_on_cancel {
      self.close();
    }
    load_dialog.close();
    Ok(())
  }

  pub fn open_or_ask(&self) {
    let app: Option<gtk4::Application> = self.application();

    if let Some(app) = app {
      if let Ok(app) = app.downcast::<MailViewerApplication>() {
        if let Some(filename) = app.imp().filename.get() {
          self.open_file(filename);
          return;
        }
      }
    }

    adw::prelude::WidgetExt::activate_action(
      self,
      "win.open-file",
      Some(&glib::Variant::from(true)),
    )
    .expect("Error opening file dialog !");
  }

  fn open_file(&self, file: &str) {
    log::debug!("open_file({})", file);
    glib::idle_add_local_once(glib::clone!(
      #[weak(rename_to = window)]
      self,
      #[strong(rename_to = filename)]
      file.to_string(),
      move || {
        match window.imp().service.open_mail(&filename) {
          Ok(_) => {
            window.display_eml();
          }
          Err(e) => {
            log::error!("service(ERR) : {}", e);
            window
              .alert_error("File Error", &format!("Failed to open file :\n{}", e))
              .connect_response(
                Some("close"),
                clone!(
                  #[strong]
                  window,
                  move |_, _| {
                    window.close();
                  }
                ),
              );
          }
        }
      }
    ));
  }

  pub fn display_eml(&self) {
    log::debug!("display_eml()");
    let imp = self.imp();

    imp.eml_from.set_text(imp.service.get_from().as_str());
    imp.eml_date.set_text(imp.service.get_date().as_str());
    imp.eml_to.set_text(imp.service.get_to().as_str());
    imp.eml_subject.set_text(imp.service.get_subject().as_str());

    let mut has_text: bool = false;
    let mut has_html: bool = false;

    if let Some(text) = imp.service.get_text() {
      imp.body_text.buffer().set_text(&text);
      has_text = true;
    }

    if let Some(html) = imp.service.get_html() {
      imp
        .web_view
        .load_html(&Html::new(&html, false).safe(), None);
      has_html = true;
    }

    if has_text && !has_html {
      self.on_show_text(true);
      imp.show_text.set_visible(false);
    } else if !has_text && has_html {
      imp.show_text.set_visible(false);
    }

    let preferences_group: adw::PreferencesGroup = adw::PreferencesGroup::new();
    self
      .imp()
      .attachments_clamp
      .set_child(Some(&preferences_group));

    let attachments = imp.service.get_attachments();
    let total = attachments.len();
    if total > 0 {
      for attachment in &attachments {
        self.add_attachment(&attachment, &preferences_group);
      }
      let label: String = format!("{} attachment{}", total, if total == 1 { "" } else { "s" });
      preferences_group.set_title(&label);
      imp.pull_label.set_text(&label);
    } else {
      // never shown
      imp.pull_label.set_text("No attachments");
    }

    if let Some(widget) = imp.sheet.bottom_bar() {
      if total > 0 {
        widget.set_visible(true)
      } else {
        widget.set_visible(false)
      }
    }
  }

  pub fn alert_error(&self, title: &str, message: &str) -> adw::AlertDialog {
    let alert = adw::AlertDialog::new(Some(title), Some(message));
    alert.add_response("close", "Close");
    alert.set_response_appearance("close", adw::ResponseAppearance::Destructive);
    alert.present(Some(self));
    alert
  }

  fn get_show_file_name(&self) -> bool {
    self
      .imp()
      .settings
      .get()
      .expect("Error settings !")
      .get::<bool>(SETTINGS_SHOW_FILE_NAME)
  }

  fn show_preferences(&self) {
    log::debug!("show_preferences()");

    let settings = self.imp().settings.get().expect("Error settings !");
    let builder = gtk4::Builder::from_resource("/io/github/alescdb/mailviewer/preferences.ui");
    let show_file_name: adw::SwitchRow = builder.object("show_file_name").unwrap();
    settings
      .bind(SETTINGS_SHOW_FILE_NAME, &show_file_name, "active")
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
          .set_show_file_name(win.get_show_file_name());
      }
    ));
  }
}
