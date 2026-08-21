use std::cell::{Cell, OnceCell};

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;

use super::capabilities::Capabilities;
use super::content::{RenderOptions, ViewerContent};
use super::printable::{Printable, PrintableContent};
use super::viewer::ViewerBackend;
use super::zoomable::Zoomable;

mod imp {
  use super::*;

  #[derive(Debug, Default, gtk4::CompositeTemplate)]
  #[template(file = "src/viewer/textview.blp")]
  pub struct TextViewer {
    #[template_child]
    pub text_view: TemplateChild<gtk4::TextView>,
    pub font_size: Cell<i32>,
    pub zoom: Cell<f64>,
    pub zoom_tag: OnceCell<gtk4::TextTag>,
  }

  #[glib::object_subclass]
  impl ObjectSubclass for TextViewer {
    type ParentType = gtk4::Box;
    type Type = super::TextViewer;

    const NAME: &'static str = "TextViewer";

    fn class_init(klass: &mut Self::Class) {
      klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
      obj.init_template();
    }
  }

  impl ObjectImpl for TextViewer {
    fn constructed(&self) {
      self.parent_constructed();

      let viewer = self.obj();
      let text_view = viewer.text_view();
      let font_size = text_view
        .pango_context()
        .font_description()
        .map(|description| description.size())
        .filter(|size| *size > 0)
        .unwrap_or(12 * gtk4::pango::SCALE);

      self.font_size.set(font_size);
      self.zoom.set(1.0);

      let zoom_tag = gtk4::TextTag::new(Some("viewer-zoom"));
      zoom_tag.set_size(font_size);
      text_view.buffer().tag_table().add(&zoom_tag);
      self.zoom_tag.set(zoom_tag).unwrap();

      let weak_viewer = viewer.downgrade();
      text_view.buffer().connect_changed(move |_| {
        if let Some(viewer) = weak_viewer.upgrade() {
          viewer.apply_zoom();
        }
      });
    }
  }
  impl WidgetImpl for TextViewer {}
  impl BoxImpl for TextViewer {}
}

glib::wrapper! {
  pub struct TextViewer(ObjectSubclass<imp::TextViewer>)
    @extends gtk4::Widget, gtk4::Box,
    @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl TextViewer {
  pub(crate) fn text_view(&self) -> &gtk4::TextView {
    &self.imp().text_view
  }

  pub(crate) fn buffer(&self) -> gtk4::TextBuffer {
    self.text_view().buffer()
  }

  fn apply_zoom(&self) {
    let Some(zoom_tag) = self.imp().zoom_tag.get() else {
      return;
    };

    let size = (self.imp().font_size.get() as f64 * self.imp().zoom.get()).round() as i32;
    zoom_tag.set_size(size.max(1));

    let buffer = self.buffer();
    buffer.apply_tag(zoom_tag, &buffer.start_iter(), &buffer.end_iter());
  }
}

impl Capabilities for TextViewer {
  fn has_text(&self) -> bool {
    true
  }

  fn has_zoom(&self) -> bool {
    true
  }
}

impl Zoomable for TextViewer {
  fn zoom(&self) -> f64 {
    self.imp().zoom.get()
  }

  fn set_zoom(&self, factor: f64) {
    self.imp().zoom.set(factor);
    self.apply_zoom();
  }
}

impl ViewerBackend for TextViewer {
  fn name(&self) -> &'static str {
    "text"
  }

  fn load_content(&self, content: &ViewerContent, _options: RenderOptions) {
    self
      .buffer()
      .set_text(content.text.as_deref().unwrap_or_default());
  }

  fn find(&self, _text: &str) {}

  fn find_next(&self) {}

  fn find_previous(&self) {}

  fn clear_find(&self) {}
}

impl Printable for TextViewer {
  fn print(
    &self,
    _parent: &gtk4::Window,
    _content: &PrintableContent,
    _options: RenderOptions,
  ) {
  }
}
