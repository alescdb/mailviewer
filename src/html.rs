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

pub struct Html {
  body: String,
  strip_css: bool,
}

impl Html {
  pub fn new(body: &str, strip_css: bool) -> Self {
    Self {
      body: body.to_string(),
      strip_css,
    }
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
    assert!(!body.contains("<meta"));
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
}
