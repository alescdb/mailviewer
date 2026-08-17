/* html.rs
 *
 * Copyright 2024 Alexandre Del Bigio
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use nipper::{Document, Node};

pub const CSS: &str = r#"
<style>
  * {
    color: black; 
    background-color: white;
    font-family: Poppins, Roboto, sans-serif;
    font-size: 20px;
  }
</style>
"#;

/// Removing tags is not enough to keep a message offline : css can reach the
/// network on its own through @import, @font-face and url(). A policy is
/// enforced by the engine, so it covers what the sanitizer cannot see.
/// data: stays allowed, it is what inline (cid) images are rewritten to.
const CSP_BLOCK_REMOTE: &str =
  "default-src 'none'; style-src 'unsafe-inline'; img-src data:; media-src data:";

const CSP_ALLOW_REMOTE: &str = "default-src 'none'; style-src 'unsafe-inline' http: https:; \
                                img-src data: http: https:; font-src http: https:; \
                                media-src data: http: https:";

pub struct Html {
  body: String,
  strip_css: bool,
  allow_remote: bool,
}

impl Html {
  pub fn new(body: &str, strip_css: bool) -> Self {
    Self {
      body: body.to_string(),
      strip_css,
      allow_remote: false,
    }
  }

  /// Whether the message may pull content from the network, i.e. the state of
  /// the "Show remote images" button.
  pub fn allow_remote(mut self, allow_remote: bool) -> Self {
    self.allow_remote = allow_remote;
    self
  }

  pub fn escape(value: &str) -> String {
    value
      .replace('&', "&amp;")
      .replace('<', "&lt;")
      .replace('>', "&gt;")
      .replace('"', "&quot;")
      .replace('\'', "&#39;")
  }

  pub fn safe(&self) -> String {
    let document = Document::from(&self.body);
    document
      .select("script,meta,audio,video,iframe,link,object,embed,applet,form")
      .iter()
      .for_each(|mut node| {
        node.remove();
      });
    self.parse(&document.root());
    if self.strip_css {
      document
        .select("html")
        .select("head")
        .first()
        .append_html(CSS);
    }
    let policy = if self.allow_remote {
      CSP_ALLOW_REMOTE
    } else {
      CSP_BLOCK_REMOTE
    };
    // As a node, not by editing the serialized html : an attribute of the head
    // can carry a '>' and swallow the tag.
    let mut head = document.select("head");
    let content = head.html();
    head.set_html(format!(
      "<meta http-equiv=\"Content-Security-Policy\" content=\"{policy}\">{content}"
    ));

    document.html().to_string()
  }

  fn parse(&self, root: &Node) {
    root.children().iter().for_each(|node| {
      if node.node_name().is_some() {
        if self.strip_css {
          node.remove_attr("style");
          node.remove_attr("class");
        }
        // Collect attribute names that start with "on"
        let attrs_to_remove: Vec<String> = node
          .attrs()
          .iter()
          .filter(|attr| Self::starts_with_on(&attr.name.local))
          .map(|attr| attr.name.local.as_ref().to_string())
          .collect();

        for attr_name in attrs_to_remove {
          node.remove_attr(&attr_name);
        }
      }
      self.parse(node);
    });
  }

  fn starts_with_on(s: &str) -> bool {
    s.len() >= 2
      && s.as_bytes()[0].eq_ignore_ascii_case(&b'o')
      && s.as_bytes()[1].eq_ignore_ascii_case(&b'n')
  }
}

#[cfg(test)]
mod tests {
  use std::error::Error;
  use std::fs;

  #[test]
  fn html() -> Result<(), Box<dyn Error>> {
    let html = crate::html::Html::new(&fs::read_to_string("tests/test.html")?, true);
    let body = html.safe().to_lowercase();

    // eprintln!("{}", &body);
    assert!(!body.contains("onblur="));
    assert!(!body.contains("onclick="));
    assert!(!body.contains("onchange="));
    assert!(!body.contains("style="));
    assert!(!body.contains("class="));

    assert!(!body.contains("<script"));
    // every meta of the message is gone, the only one left is our own policy
    assert_eq!(body.matches("<meta").count(), 1);
    assert!(body.contains("<meta http-equiv=\"content-security-policy\""));
    assert!(!body.contains("<audio"));
    assert!(!body.contains("<video"));
    assert!(!body.contains("<iframe"));
    assert!(!body.contains("<link"));
    assert!(!body.contains("<object"));
    assert!(!body.contains("<embed"));
    assert!(!body.contains("<applet"));
    assert!(!body.contains("<form"));

    assert!(body.contains(&crate::html::CSS.to_lowercase()));

    Ok(())
  }

  #[test]
  fn csp_is_the_first_thing_in_the_head() {
    let body = crate::html::Html::new("<p>hello</p>", false).safe();
    let head = body.find("<head>").expect("a head is always serialized");

    assert!(body[head..].starts_with(
      "<head><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none';"
    ));
  }

  #[test]
  fn csp_blocks_remote_content_by_default() {
    let body = crate::html::Html::new("<p>hello</p>", false).safe();

    assert!(body.contains("img-src data:;"));
    assert!(!body.contains("img-src data: http:"));
    assert!(!body.contains("font-src"));
  }

  #[test]
  fn csp_opens_up_when_remote_content_is_allowed() {
    let body = crate::html::Html::new("<p>hello</p>", false)
      .allow_remote(true)
      .safe();

    assert!(body.contains("img-src data: http: https:;"));
    assert!(body.contains("font-src http: https:;"));
  }

  #[test]
  fn csp_is_not_swallowed_by_an_attribute_of_the_head() {
    // A '>' inside an attribute of the head used to end the tag as far as a
    // textual insertion was concerned, and the meta landed in the attribute.
    let body = crate::html::Html::new(
      "<html><head title=\">\"><style>@import url(http://tracker/x.css);</style></head><body>hi</body></html>",
      false,
    )
    .safe();

    assert!(
      body.contains("<head title=\">\"><meta http-equiv=\"Content-Security-Policy\""),
      "the policy must be an element of the head, not part of an attribute: {body}"
    );
  }

  #[test]
  fn csp_survives_a_message_that_brings_its_own_head() {
    let body = crate::html::Html::new(
      "<html><head><style>@import url(http://tracker/x.css);</style></head><body>hi</body></html>",
      false,
    )
    .safe();
    let head = body.find("<head>").unwrap();
    let meta = body.find("Content-Security-Policy").unwrap();
    let style = body.find("<style>").unwrap();

    assert!(head < meta && meta < style, "the policy must come first");
  }
}
