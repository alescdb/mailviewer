// Common utilities.
#[cfg(test)]
use crate::glib;

/// Returns the lowercased scheme of `uri`, or None if it has no valid one.
/// A scheme is an ASCII letter followed by letters, digits, '+', '-' or '.'
/// (RFC 3986).
pub fn uri_scheme(uri: &str) -> Option<String> {
  let scheme = uri.split(':').next()?;
  if scheme.is_empty() || scheme.len() == uri.len() {
    return None;
  }

  let mut chars = scheme.chars();
  if !chars.next()?.is_ascii_alphabetic() {
    return None;
  }
  if !chars.all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.') {
    return None;
  }

  Some(scheme.to_ascii_lowercase())
}

#[cfg(test)]
pub fn spawn_and_wait<R: 'static, F: std::future::Future<Output = R> + 'static>(
  ctx: Option<&glib::MainContext>,
  f: F,
) -> R {
  let ctx = match ctx {
    Some(ctx) => ctx,
    None => &glib::MainContext::default(),
  };
  use std::any::Any;
  use std::cell::RefCell;
  use std::rc::Rc;

  use futures_util::FutureExt;

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
  use gio::prelude::*;

  use crate::utils::*;
  use crate::{gio, glib};

  #[test]
  fn uri_schemes() {
    assert_eq!(uri_scheme("https://example.com").as_deref(), Some("https"));
    assert_eq!(uri_scheme("HTTPS://example.com").as_deref(), Some("https"));
    assert_eq!(
      uri_scheme("mailto:john@moon.space").as_deref(),
      Some("mailto")
    );
    assert_eq!(
      uri_scheme("javascript:alert(1)").as_deref(),
      Some("javascript")
    );
    assert_eq!(uri_scheme("file:///etc/passwd").as_deref(), Some("file"));
    assert_eq!(uri_scheme(""), None);
    assert_eq!(uri_scheme("no-scheme"), None);
    assert_eq!(uri_scheme("://example.com"), None);
    assert_eq!(uri_scheme("1http://example.com"), None);
  }

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
