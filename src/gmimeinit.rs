use ctor::{ctor, dtor};
#[ctor]
fn initialize_gmime() {
  unsafe {
    gmime::ffi::g_mime_init();
  }
}

#[dtor]
fn shutdown_gmime() {
  unsafe {
    gmime::ffi::g_mime_shutdown();
  }
}
