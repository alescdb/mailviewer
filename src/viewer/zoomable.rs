pub trait Zoomable {
  fn zoom(&self) -> f64;
  fn set_zoom(&self, factor: f64);
}
