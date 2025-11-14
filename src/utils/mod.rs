// Common utilities.
use crate::glib;

pub fn spawn_and_wait<R: 'static, F: std::future::Future<Output = R> + 'static>(
  ctx: Option<&glib::MainContext>,
  f: F,
) -> R {
  let ctx = match ctx {
    Some(ctx) => ctx,
    None => &glib::MainContext::default(),
  };
  use futures_util::FutureExt;
  use std::cell::RefCell;
  use std::rc::Rc;
  use std::any::Any;

  let lp = glib::MainLoop::new(Some(ctx), false);
  let ret = Rc::new(RefCell::new(None::<Result<R, Box<dyn Any + Send>>>));

  ctx.spawn_local(glib::clone!(
    #[strong]
    lp,
    #[weak]
    ret,
    async move {
      *ret.borrow_mut() = Some(std::panic::AssertUnwindSafe(f).catch_unwind().await);
      lp.quit();
    }
  ));

  lp.run();

  match ret.take().unwrap() {
    Ok(r) => r,
    Err(e) => std::panic::resume_unwind(Box::new(e)),
  }
}

#[cfg(test)]
pub fn spawn_and_wait_new_ctx<R: 'static, F: std::future::Future<Output = R> + 'static>(f: F) {
  spawn_and_wait(Some(&glib::MainContext::new()), f);
}

#[cfg(test)]
mod tests {
  use crate::{glib, gio};
  use crate::utils::*;

  use gio::prelude::*;

  #[test]
  fn wait_for_no_result() {
    assert_eq!(
      spawn_and_wait(Some(&glib::MainContext::new()), async move {
        glib::timeout_future(std::time::Duration::from_millis(100)).await;
      }),
      ()
    );
  }

  #[test]
  fn wait_for_some_value() {
    assert_eq!(
      spawn_and_wait(Some(&glib::MainContext::new()), async move {
        glib::timeout_future(std::time::Duration::from_millis(100)).await;
        12345
      }),
      12345
    );
  }

  #[test]
  fn wait_for_some_result() {
    assert_eq!(
      spawn_and_wait(Some(&glib::MainContext::new()), async move {
        glib::timeout_future(std::time::Duration::from_millis(100)).await;
        Ok::<glib::Variant, glib::Error>("foobar".to_variant())
      })
      .unwrap(),
      "foobar".to_variant()
    );
  }

  #[test]
  fn wait_for_some_result_error() {
    assert!(spawn_and_wait(Some(&glib::MainContext::new()), async move {
      glib::timeout_future(std::time::Duration::from_millis(100)).await;
      Err::<String, glib::Error>(glib::Error::new(glib::UriError::BadHost, "an error"))
    })
    .unwrap_err()
    .matches(glib::UriError::BadHost));
  }

  #[test]
  #[should_panic]
  fn wait_for_panicking() {
    spawn_and_wait(Some(&glib::MainContext::new()), async move {
      glib::timeout_future(std::time::Duration::from_millis(100)).await;
      panic!("so sad!");
    });
  }

  #[test]
  #[should_panic]
  fn wait_for_panicking_future() {
    spawn_and_wait(Some(&glib::MainContext::new()), async move {
      let panicking = async {
        glib::timeout_future(std::time::Duration::from_millis(100)).await;
        panic!("so sad!");
      };
      panicking.await;
    })
  }
}
