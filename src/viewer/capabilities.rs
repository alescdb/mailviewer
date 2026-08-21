#[allow(dead_code)]
pub trait Capabilities {
  fn has_html(&self) -> bool {
    false
  }

  fn has_text(&self) -> bool {
    false
  }

  fn has_zoom(&self) -> bool {
    false
  }

  fn has_images(&self) -> bool {
    false
  }

  fn has_links(&self) -> bool {
    false
  }

  fn has_search(&self) -> bool {
    false
  }

  fn has_print(&self) -> bool {
    false
  }

  fn has_network(&self) -> bool {
    false
  }
}
