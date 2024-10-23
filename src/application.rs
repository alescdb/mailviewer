use adw::subclass::prelude::*;
use gtk4::{gio, glib, prelude::*};

use crate::{
  config::{APP_ID, VERSION}, mailparser::MailParser, MailViewerWindow
};
use adw::prelude::AdwDialogExt;

mod imp {
  use super::*;
  use crate::mailparser::MailParser;
  use std::cell::OnceCell;

  #[derive(Debug, Default)]
  pub struct MailViewerApplication {
    pub window: OnceCell<MailViewerWindow>,
    pub parser: OnceCell<MailParser>,
    pub file: OnceCell<String>,
  }

  #[glib::object_subclass]
  impl ObjectSubclass for MailViewerApplication {
    const NAME: &'static str = "MailViewerApplication";
    type Type = super::MailViewerApplication;
    type ParentType = adw::Application;
    type Interfaces = ();
  }

  impl ObjectImpl for MailViewerApplication {
    fn constructed(&self) {
      self.parent_constructed();
      let obj = self.obj();
      obj.setup_gactions();
      obj.set_accels_for_action("app.quit", &["<primary>q"]);
    }
  }

  impl ApplicationImpl for MailViewerApplication {
    fn activate(&self) {
      let application = self.obj();

      let window: MailViewerWindow = if let Some(window) = application.active_window() {
        window.downcast::<MailViewerWindow>().ok().unwrap()
      } else {
        let window = MailViewerWindow::new(&*application);
        window.upcast()
      };
      self
        .window
        .set(window.clone())
        .expect("Window already set.");

      let provider = gtk4::CssProvider::new();
      provider.load_from_resource("/org/cosinus/mailviewer/css/style.css");

      // Appliquer le CSS à l'écran par défaut
      if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
          &display,
          &provider,
          gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
      }
      window.present();

      self.obj().load_eml(&window);
    }

    fn open(&self, files: &[gio::File], hint: &str) {
      for file in files {
        log::debug!("[ARGUMENT] File: {:?}, Hint : {:?}", file.path(), hint);
      }

      if files.is_empty() == false {
        if let Some(path) = files[0].path() {
          self
            .file
            .set(path.to_str().unwrap().to_string())
            .expect("File already initialized.");
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

  fn load_eml(&self, window: &MailViewerWindow) {
    let app = self;
    glib::idle_add_local_once(glib::clone!(
      #[weak]
      window,
      #[weak]
      app,
      move || {
        println!("Thread eml {:?}", window);
        app.open_eml_file().unwrap_or_else(|err| {
          log::error!("Error : {}", err);
        });
        window.emit_by_name::<()>("eml-parsed", &[]);
      }
    ));
  }

  fn open_eml_file(&self) -> Result<(), std::boxed::Box<dyn std::error::Error>> {
    let imp = self.imp();
    if let Some(file) = imp.file.get() {
      let mut parser = MailParser::new(&file);
      parser.parse()?;
      match imp.parser.set(parser) {
        Ok(it) => it,
        Err(_) => return Err("Failed to set parser".into()),
      };
    }
    Ok(())
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
      .developers(vec!["Alexandre Del Bigio"])
      .issue_url("https://github.com/alescdb")
      .support_url("https://github.com/alescdb")
      .release_notes_version("2.3.0")
      .build();

    dialog.add_link("GitHub", "https://github.com/alescdb");
    dialog.present(Some(&window));
  }
}
