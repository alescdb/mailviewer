// Test utilities used by unit tests.
#[cfg(test)]
use std::sync::mpsc::channel;

use futures_util::FutureExt;

use crate::glib;

pub fn spawn_and_wait<R: 'static, F: std::future::Future<Output = R> + 'static>(f: F) {
  let ctx = glib::MainContext::new();
  let lp = glib::MainLoop::new(Some(&ctx), false);

  let test_lp = lp.clone();
  let (tx, rx) = channel::<Result<(), ()>>();

  ctx.spawn_local(async move {
    let _ = match std::panic::AssertUnwindSafe(f).catch_unwind().await {
      Ok(_) => tx.send(Ok(())),
      Err(_) => tx.send(Err(())),
    };
    test_lp.quit();
  });

  lp.run();
  assert!(rx.recv().unwrap().is_ok());
}
