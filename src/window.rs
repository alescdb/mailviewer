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
use gtk4::prelude::FileChooserExt;
use gtk4::{gio, glib, template_callbacks, ResponseType};
use webkit6::prelude::{PolicyDecisionExt, WebViewExt};
use webkit6::{NavigationPolicyDecision, PolicyDecision, PolicyDecisionType, WebView};

use crate::html::Html;
use crate::mailservice::MailService;
use crate::message::attachment::Attachment;
use crate::message::message::MessageParser;

const SETTINGS_SHOW_FILE_NAME: &str = "show-file-name";

mod imp {
  use std::cell::OnceCell;

  use adw::subclass::prelude::CompositeTemplateClass;
  use gtk4::ScrolledWindow;

  use super::*;

  #[derive(Debug, gtk4::CompositeTemplate)]
  #[template(resource = "/io/github/alescdb/mailviewer/window.ui")]
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
    pub webview: webkit6::WebView,
    pub websettings: webkit6::Settings,
    pub settings: OnceCell<gio::Settings>,
    pub service: MailService,
  }

  impl Default for MailViewerWindow {
    fn default() -> Self {
      let window = MailViewerWindow {
        webview: WebView::new(),
        websettings: webkit6::Settings::new(),
        scrolled_window: ScrolledWindow::new(),
        from: TemplateChild::default(),
        to: TemplateChild::default(),
        subject: TemplateChild::default(),
        date: TemplateChild::default(),
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
    type Interfaces = ();
    type ParentType = adw::ApplicationWindow;
    type Type = super::MailViewerWindow;

    const ABSTRACT: bool = false;
    const NAME: &'static str = "MailViewerWindow";

    fn class_init(klass: &mut Self::Class) {
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
    self.imp().websettings.set_auto_load_images(show);
  }

  #[template_callback]
  pub fn on_zoom_minus_clicked(&self) {
    log::debug!("on_zoom_minus_clicked()");
    self.set_zoom_level(self.imp().webview.zoom_level() - 0.1);
  }

  #[template_callback]
  pub fn on_zoom_plus_clicked(&self) {
    log::debug!("on_zoom_plus_clicked()");
    self.set_zoom_level(self.imp().webview.zoom_level() + 0.1);
  }

  fn initialize(&self) {
    log::debug!("initialize()");
    let imp = self.imp();

    self.initialize_settings();
    self.initialize_actions();

    imp.websettings.set_allow_file_access_from_file_urls(false);
    imp
      .websettings
      .set_enable_back_forward_navigation_gestures(false);
    imp.websettings.set_enable_developer_extras(false);
    imp.websettings.set_enable_dns_prefetching(false);
    imp.websettings.set_allow_modal_dialogs(false);
    imp
      .websettings
      .set_allow_universal_access_from_file_urls(false);
    imp.websettings.set_enable_javascript(false);
    imp.websettings.set_auto_load_images(false);
    imp.webview.set_settings(&imp.websettings);
    imp.webview.set_editable(false);
    imp.webview.connect_context_menu(move |_, _, _| {
      log::debug!("WebView() => context_menu() cancelled");
      true
    });
    imp.webview.set_receives_default(false);
    imp.placeholder.set_child(Some(&imp.webview));
  }

  fn initialize_actions(&self) {
    let win = self;
    let imp = self.imp();

    let drop_target = gtk4::DropTarget::new(gio::File::static_type(), gtk4::gdk::DragAction::COPY);
    imp.body_text.add_controller(drop_target.clone());
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

    imp.webview.connect_decide_policy(clone!(
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
    imp.webview.set_zoom_level(settings.get::<f64>("zoom"));

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
      .child(attachment.filename.as_str());

    let save_dialog = gtk4::FileDialog::builder()
      .title(&gettext("Save attachment..."))
      .modal(true)
      .initial_file(&initial_file)
      .build();

    match save_dialog.save_future(Some(self)).await {
      Ok(file) => {
        let path = file.peek_path().unwrap_or_default();
        let path = path.display();
        log::debug!("Saving attachment to {:?}", &path);
        match attachment.write_to_file(&file).await {
          Ok(_) => log::debug!("write_to_file({:?})", &path),
          Err(e) => {
            log::error!("write_to_file({})", e);
            self.alert_error(&gettext("File Error"), &e.to_string(), false);
          }
        };
      }
      Err(e) => match e.kind() {
        Some(gtk4::DialogError::Dismissed) | Some(gtk4::DialogError::Cancelled) => return,
        _ => log::error!("save_dialog({})", e),
      },
    }
  }

  async fn on_attachment_open(&self, attachment: &Attachment) {
    log::debug!("on_button_clicked({})", attachment.filename);
    match attachment.write_to_tmp().await {
      Ok(file) => {
        log::debug!("write_to_tmp({}) success", &file);
        if let Err(e) = open::that(&file) {
          log::error!("{} ({}): {}", &gettext("Failed to open file"), &file, e);
        }
      }
      Err(e) => log::error!("write_to_tmp({})", e),
    };
  }

  fn set_zoom_level(&self, zoom: f64) {
    log::debug!("set_zoom({})", zoom);
    self.imp().webview.set_zoom_level(zoom);
    if let Some(settings) = self.imp().settings.get() {
      let _ = settings.set("zoom", zoom);
    }
  }

  fn load_html(&self, force_css: bool) {
    log::debug!("load_html({})", force_css);
    let html = self.imp().service.body_html().unwrap_or(String::new());
    self
      .imp()
      .webview
      .load_html(&*Html::new(&html, force_css).safe(), None);
  }

  fn decide_policy(&self, policy: &PolicyDecision) -> Result<bool, Box<dyn std::error::Error>> {
    match policy.clone().downcast::<NavigationPolicyDecision>() {
      Ok(policy) => {
        let navigation_action = policy.navigation_action();
        if let Some(navigation_action) = navigation_action {
          if let Some(request) = navigation_action.clone().request() {
            if let Some(uri) = request.uri() {
              if uri.starts_with("about:") {
                return Ok(false);
              }
              log::debug!("WebView on_decide_policy(open) => {}", uri);
              open::that(uri.to_string())?;
            }
            policy.ignore();
            return Ok(true);
          }
        }
      }
      Err(e) => {
        log::error!("WebView policy.clone().downcast({:?})", e);
        return Err(format!("on_decide_policy() policy downcast failed ({:?})", e).into());
      }
    }
    Ok(false)
  }

  fn on_decide_policy(
    &self,
    _: &WebView,
    policy: &PolicyDecision,
    _decision_type: PolicyDecisionType,
  ) -> bool {
    match self.decide_policy(policy) {
      Ok(res) => res,
      Err(e) => {
        log::error!("WebView on_decide_policy({:?})", e);
        false
      }
    }
  }

  fn on_show_text(&self, show: bool) {
    log::debug!("on_show_text({})", show);
    let imp = self.imp();

    imp
      .stack
      .get()
      .set_visible_child_name(if show { "text" } else { "html" });

    if imp.show_text.is_active() != show {
      imp.show_text.set_active(show);
    }
    imp.show_images.set_visible(!show);
    imp.force_css.set_visible(!show);
    imp.zoom_minus.set_visible(!show);
    imp.zoom_plus.set_visible(!show);
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
    return gtk4::FileDialog::builder()
      .title(title)
      .modal(true)
      .filters(&filters)
      .build();
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

    match self.imp().service.open_message(&file).await {
      Ok(_) => {
        self.display_message();
      }
      Err(e) => {
        log::error!("service(ERR) : {}", e);
        self.alert_error(
          &gettext("File Error"),
          &format!("{}:\n{}", &gettext("Failed to open file"), e),
          true,
        );
      }
    }
  }

  pub fn display_message(&self) {
    log::debug!("display_eml()");
    let imp = self.imp();

    imp.from.set_text(imp.service.from().as_str());
    imp.date.set_text(imp.service.date().as_str());
    imp.to.set_text(imp.service.to().as_str());
    imp.subject.set_text(imp.service.subject().as_str());

    let mut has_text: bool = false;
    let mut has_html: bool = false;

    if let Some(text) = imp.service.body_text() {
      imp.body_text.buffer().set_text(&text);
      has_text = true;
    }

    if let Some(html) = imp.service.body_html() {
      imp.webview.load_html(&Html::new(&html, false).safe(), None);
      has_html = true;
    }

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
        self.add_attachment(&attachment, &preferences_group);
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

  fn get_show_file_name(&self) -> bool {
    if let Some(settings) = self.imp().settings.get() {
      settings.get::<bool>(SETTINGS_SHOW_FILE_NAME)
    } else {
      false
    }
  }

  fn show_preferences(&self) {
    log::debug!("show_preferences()");
    match self.imp().settings.get() {
      Some(settings) => {
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
