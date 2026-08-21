use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

use adw::subclass::prelude::*;
use gtk4::glib;
use gtk4::prelude::*;

use super::capabilities::Capabilities;
use super::content::{RenderOptions, ViewerContent};
use super::printable::{Printable, PrintableContent};
use super::textview::TextViewer;
use super::webview::WebViewer;
use super::zoomable::Zoomable;

pub(crate) trait ViewerBackend: Capabilities + Printable + Zoomable {
  fn name(&self) -> &'static str;
  fn load_content(&self, content: &ViewerContent, options: RenderOptions);
  fn find(&self, text: &str);
  fn find_next(&self);
  fn find_previous(&self);
  fn clear_find(&self);
  fn connect_link_handler(&self, _handler: Rc<dyn Fn(String) + 'static>) {}
}

mod imp {
  use super::*;

  #[derive(gtk4::CompositeTemplate)]
  #[template(file = "src/viewer/viewer.blp")]
  pub struct Viewer {
    #[template_child]
    pub stack: TemplateChild<adw::ViewStack>,
    #[template_child]
    pub web_view: TemplateChild<WebViewer>,
    #[template_child]
    pub text_view: TemplateChild<TextViewer>,
    pub(crate) backends: OnceCell<Vec<Box<dyn ViewerBackend>>>,
    pub(crate) content: RefCell<Option<ViewerContent>>,
    pub(crate) options: Cell<RenderOptions>,
  }

  impl Default for Viewer {
    fn default() -> Self {
      Self {
        stack: TemplateChild::default(),
        web_view: TemplateChild::default(),
        text_view: TemplateChild::default(),
        backends: OnceCell::new(),
        content: RefCell::new(None),
        options: Cell::new(RenderOptions {
          force_css: false,
          allow_remote: false,
          dark: false,
        }),
      }
    }
  }

  #[glib::object_subclass]
  impl ObjectSubclass for Viewer {
    type ParentType = adw::Bin;
    type Type = super::Viewer;

    const NAME: &'static str = "Viewer";

    fn class_init(klass: &mut Self::Class) {
      WebViewer::static_type();
      TextViewer::static_type();
      klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
      obj.init_template();
    }
  }

  impl ObjectImpl for Viewer {
    fn constructed(&self) {
      self.parent_constructed();

      let viewer = self.obj();
      let web_view = viewer.imp().web_view.get();
      let text_view = viewer.imp().text_view.get();
      let _ = viewer.imp().backends.set(vec![
        Box::new(web_view) as Box<dyn ViewerBackend>,
        Box::new(text_view) as Box<dyn ViewerBackend>,
      ]);
    }
  }
  impl WidgetImpl for Viewer {}
  impl BinImpl for Viewer {}
}

glib::wrapper! {
  pub struct Viewer(ObjectSubclass<imp::Viewer>)
    @extends gtk4::Widget, adw::Bin,
    @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl Viewer {
  fn stack(&self) -> &adw::ViewStack {
    &self.imp().stack
  }

  fn backends(&self) -> &[Box<dyn ViewerBackend>] {
    self.imp().backends.get().unwrap()
  }

  fn active_backend(&self) -> Option<&dyn ViewerBackend> {
    let name = self.stack().visible_child_name();
    self
      .backends()
      .iter()
      .find(|backend| Some(backend.name()) == name.as_deref())
      .map(|backend| backend.as_ref())
  }

  pub(crate) fn load_content(&self, content: ViewerContent, options: RenderOptions) {
    self.imp().content.replace(Some(content));
    self.set_render_options(options);
  }

  pub(crate) fn set_render_options(&self, options: RenderOptions) {
    self.imp().options.set(options);
    let content = self.imp().content.borrow();
    let Some(content) = content.as_ref() else {
      return;
    };

    for backend in self.backends() {
      backend.load_content(content, options);
    }
  }

  pub(crate) fn connect_link_handler<F: Fn(String) + 'static>(&self, handler: F) {
    let handler: Rc<dyn Fn(String) + 'static> = Rc::new(handler);
    for backend in self.backends() {
      backend.connect_link_handler(handler.clone());
    }
  }

  pub(crate) fn set_text_visible(&self, visible: bool) {
    self
      .stack()
      .set_visible_child_name(if visible { "text" } else { "html" });
  }

  pub(crate) fn find(&self, text: &str) {
    if let Some(backend) = self.active_backend() {
      backend.find(text);
    }
  }

  pub(crate) fn find_next(&self) {
    if let Some(backend) = self.active_backend() {
      backend.find_next();
    }
  }

  pub(crate) fn find_previous(&self) {
    if let Some(backend) = self.active_backend() {
      backend.find_previous();
    }
  }

  pub(crate) fn clear_find(&self) {
    if let Some(backend) = self.active_backend() {
      backend.clear_find();
    }
  }


  fn active_capabilities(&self) -> &dyn Capabilities {
    self.active_backend().unwrap()
  }
}

impl Capabilities for Viewer {
  fn has_html(&self) -> bool {
    self.active_capabilities().has_html()
  }

  fn has_text(&self) -> bool {
    self.active_capabilities().has_text()
  }

  fn has_zoom(&self) -> bool {
    self.active_capabilities().has_zoom()
  }

  fn has_images(&self) -> bool {
    self.active_capabilities().has_images()
  }

  fn has_links(&self) -> bool {
    self.active_capabilities().has_links()
  }

  fn has_search(&self) -> bool {
    self.active_capabilities().has_search()
  }

  fn has_print(&self) -> bool {
    self.active_capabilities().has_print()
  }

  fn has_network(&self) -> bool {
    self.active_capabilities().has_network()
  }
}

impl Printable for Viewer {
  fn print(
    &self,
    parent: &gtk4::Window,
    content: &PrintableContent,
    options: RenderOptions,
  ) {
    if let Some(backend) = self.active_backend() {
      backend.print(parent, content, options);
    }
  }
}

impl Zoomable for Viewer {
  fn zoom(&self) -> f64 {
    self.active_backend().map_or(1.0, Zoomable::zoom)
  }

  fn set_zoom(&self, factor: f64) {
    for backend in self.backends() {
      backend.set_zoom(factor);
    }
  }
}
