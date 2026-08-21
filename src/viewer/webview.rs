use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::html::Html;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use webkit6::prelude::*;
use webkit6::{
  FindOptions, NavigationPolicyDecision, PrintOperation, PrintOperationResponse,
  WebView as GtkWebView,
};

use super::capabilities::Capabilities;
use super::content::{RenderOptions, ViewerContent};
use super::printable::{Printable, PrintableContent};
use super::viewer::ViewerBackend;
use super::zoomable::Zoomable;

mod imp {
  use super::*;

  #[derive(Debug, gtk4::CompositeTemplate)]
  #[template(file = "src/viewer/webview.blp")]
  pub struct WebViewer {
    #[template_child]
    pub scrolled_window: TemplateChild<gtk4::ScrolledWindow>,
    pub network_session: webkit6::NetworkSession,
    pub webview: GtkWebView,
    pub settings: webkit6::Settings,
    pub zoom: Cell<f64>,
    pub print_webview: RefCell<Option<GtkWebView>>,
    pub print_operation: RefCell<Option<PrintOperation>>,
  }

  impl Default for WebViewer {
    fn default() -> Self {
      let network_session = webkit6::NetworkSession::new_ephemeral();
      let webview = GtkWebView::builder()
        .network_session(&network_session)
        .build();

      Self {
        scrolled_window: TemplateChild::default(),
        network_session,
        webview,
        settings: webkit6::Settings::new(),
        zoom: Cell::new(1.0),
        print_webview: RefCell::new(None),
        print_operation: RefCell::new(None),
      }
    }
  }

  #[glib::object_subclass]
  impl ObjectSubclass for WebViewer {
    type ParentType = gtk4::Box;
    type Type = super::WebViewer;

    const NAME: &'static str = "WebViewer";

    fn class_init(klass: &mut Self::Class) {
      klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
      obj.init_template();
    }
  }

  impl ObjectImpl for WebViewer {
    fn constructed(&self) {
      self.parent_constructed();

      let viewer = self.obj();
      viewer.configure_webview(&viewer.imp().webview, &viewer.imp().settings, false);
      self.scrolled_window.set_child(Some(&viewer.imp().webview));
    }
  }

  impl WidgetImpl for WebViewer {}
  impl BoxImpl for WebViewer {}
}

glib::wrapper! {
  pub struct WebViewer(ObjectSubclass<imp::WebViewer>)
    @extends gtk4::Widget, gtk4::Box,
    @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl WebViewer {
  pub(crate) fn webview(&self) -> &GtkWebView {
    &self.imp().webview
  }

  pub(crate) fn network_session(&self) -> webkit6::NetworkSession {
    self.imp().network_session.clone()
  }

  pub(crate) fn load_html(&self, html: &str) {
    self.webview().load_html(html, None);
  }

  pub(crate) fn find(&self, text: &str) {
    let Some(controller) = self.webview().find_controller() else {
      return;
    };

    if text.is_empty() {
      controller.search_finish();
      return;
    }

    controller.search(
      text,
      (FindOptions::CASE_INSENSITIVE | FindOptions::WRAP_AROUND).bits(),
      u32::MAX,
    );
  }

  pub(crate) fn find_next(&self) {
    if let Some(controller) = self.webview().find_controller() {
      controller.search_next();
    }
  }

  pub(crate) fn find_previous(&self) {
    if let Some(controller) = self.webview().find_controller() {
      controller.search_previous();
    }
  }

  pub(crate) fn clear_find(&self) {
    if let Some(controller) = self.webview().find_controller() {
      controller.search_finish();
    }
  }

  pub(crate) fn configure_webview(
    &self,
    webview: &GtkWebView,
    settings: &webkit6::Settings,
    images_enabled: bool,
  ) {
    settings.set_allow_file_access_from_file_urls(false);
    settings.set_enable_back_forward_navigation_gestures(false);
    settings.set_enable_developer_extras(false);
    settings.set_enable_dns_prefetching(false);
    settings.set_allow_modal_dialogs(false);
    settings.set_allow_universal_access_from_file_urls(false);
    settings.set_enable_javascript(false);
    settings.set_enable_webgl(false);
    settings.set_enable_webaudio(false);
    settings.set_auto_load_images(images_enabled);
    webview.set_settings(settings);
    webview.set_editable(false);
    webview.connect_context_menu(move |_, _, _| {
      log::debug!("WebView() => context_menu() cancelled");
      true
    });
    webview.set_receives_default(false);
  }

  pub(crate) fn print(
    &self,
    parent: &gtk4::Window,
    content: &PrintableContent,
    options: RenderOptions,
  ) {
    let network_session = self.network_session();
    let webview = GtkWebView::builder()
      .network_session(&network_session)
      .build();
    let settings = webkit6::Settings::new();
    self.configure_webview(&webview, &settings, options.allow_remote);
    self.imp().print_webview.replace(Some(webview.clone()));

    let weak_viewer = self.downgrade();
    let weak_parent = parent.downgrade();
    webview.connect_load_changed(move |webview, event| {
      if event != webkit6::LoadEvent::Finished {
        return;
      }

      let print_operation = PrintOperation::new(webview);
      let failed_viewer = weak_viewer.clone();
      print_operation.connect_failed(move |_, error| {
        log::error!("print failed: {}", error);
        if let Some(viewer) = failed_viewer.upgrade() {
          viewer.imp().print_operation.replace(None);
          viewer.imp().print_webview.replace(None);
        }
      });

      let finished_viewer = weak_viewer.clone();
      print_operation.connect_finished(move |_| {
        log::debug!("print finished");
        if let Some(viewer) = finished_viewer.upgrade() {
          viewer.imp().print_operation.replace(None);
          viewer.imp().print_webview.replace(None);
        }
      });

      let Some(viewer) = weak_viewer.upgrade() else {
        return;
      };
      viewer
        .imp()
        .print_operation
        .replace(Some(print_operation.clone()));

      let Some(parent) = weak_parent.upgrade() else {
        viewer.imp().print_operation.replace(None);
        viewer.imp().print_webview.replace(None);
        return;
      };

      let response = print_operation.run_dialog(Some(&parent));
      if response == PrintOperationResponse::Print {
        log::debug!("print started");
      } else {
        log::debug!("print cancelled");
        viewer.imp().print_operation.replace(None);
        viewer.imp().print_webview.replace(None);
      }
    });
    let body = if let Some(html) = content.html.as_deref() {
      html.to_string()
    } else if let Some(text) = content.text.as_deref() {
      format!("<pre>{}</pre>", Html::escape(text))
    } else {
      String::new()
    };
    let html = Html::new(&body, false)
      .allow_remote(options.allow_remote)
      .inline_images(&content.attachments)
      .safe_print(
        &content.from,
        &content.to,
        &content.date,
        &content.subject,
        &content.attachments,
      );
    webview.load_html(&html, None);
  }
}

impl Capabilities for WebViewer {
  fn has_html(&self) -> bool {
    true
  }

  fn has_zoom(&self) -> bool {
    true
  }

  fn has_images(&self) -> bool {
    true
  }

  fn has_links(&self) -> bool {
    true
  }

  fn has_search(&self) -> bool {
    true
  }

  fn has_print(&self) -> bool {
    true
  }

  fn has_network(&self) -> bool {
    true
  }
}

impl Zoomable for WebViewer {
  fn zoom(&self) -> f64 {
    self.imp().zoom.get()
  }

  fn set_zoom(&self, factor: f64) {
    self.imp().zoom.set(factor);
    self.webview().set_zoom_level(factor);
  }
}

impl ViewerBackend for WebViewer {
  fn name(&self) -> &'static str {
    "html"
  }

  fn load_content(&self, content: &ViewerContent, options: RenderOptions) {
    self.imp().settings.set_auto_load_images(options.allow_remote);
    let html = content.html.as_deref().unwrap_or_default();
    let html = Html::new(html, options.force_css)
      .allow_remote(options.allow_remote)
      .dark(options.dark)
      .inline_images(&content.attachments)
      .safe();
    self.load_html(&html);
  }

  fn find(&self, text: &str) {
    WebViewer::find(self, text);
  }

  fn find_next(&self) {
    WebViewer::find_next(self);
  }

  fn find_previous(&self) {
    WebViewer::find_previous(self);
  }

  fn clear_find(&self) {
    WebViewer::clear_find(self);
  }

  fn connect_link_handler(&self, handler: Rc<dyn Fn(String) + 'static>) {
    self.webview().connect_decide_policy(move |_, policy, _| {
      let Ok(policy) = policy.clone().downcast::<NavigationPolicyDecision>() else {
        return false;
      };

      let Some(action) = policy.navigation_action() else {
        policy.ignore();
        return true;
      };
      let Some(request) = action.request() else {
        policy.ignore();
        return true;
      };
      let Some(uri) = request.uri() else {
        policy.ignore();
        return true;
      };
      if uri.starts_with("about:") {
        return false;
      }

      policy.ignore();
      handler(uri.to_string());
      true
    });
  }
}

impl Printable for WebViewer {
  fn print(
    &self,
    parent: &gtk4::Window,
    content: &PrintableContent,
    options: RenderOptions,
  ) {
    WebViewer::print(self, parent, content, options);
  }
}
