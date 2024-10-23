use nipper::{Document, Node};

const CSS: &str = r#"
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
      document.select("html").first().append_html(CSS);
    }
    document.html().to_string()
  }

  pub fn parse(&self, root: &Node) {
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
  use super::*;

  #[test]
  fn test_safe_removes_unwanted_elements() {
    let input_html = r#"
            <html>
                <head>
                    <script>alert("hack");</script>
                    <meta charset="UTF-8">
                </head>
                <body>
                    <h1>Hello, world!</h1>
                    <iframe src="bad.html"></iframe>
                </body>
            </html>
        "#;

    let html = Html::new(input_html, false);
    let result = html.safe();

    assert!(!result.contains("<script>"));
    assert!(!result.contains("<meta"));
    assert!(!result.contains("<iframe"));
    assert!(result.contains("<h1>Hello, world!</h1>"));
  }

  #[test]
  fn test_safe_with_strip_css_removes_inline_styles_and_classes() {
    let input_html = r#"
            <html>
                <body>
                    <h1 style="color:red;" class="title">Hello, world!</h1>
                </body>
            </html>
        "#;

    let html = Html::new(input_html, true);
    let result = html.safe();

    assert!(!result.contains("style="));
    assert!(!result.contains("class="));
    assert!(result.contains("<h1>Hello, world!</h1>"));
  }

  #[test]
  fn test_safe_adds_css_when_strip_css_is_true() {
    let input_html = r#"
            <html>
                <body>
                    <h1>Hello, world!</h1>
                </body>
            </html>
        "#;

    let html = Html::new(input_html, true);
    let result = html.safe();

    assert!(result.contains(CSS));
  }

  #[test]
  fn test_safe_does_not_add_css_when_strip_css_is_false() {
    let input_html = r#"
            <html>
                <body>
                    <h1>Hello, world!</h1>
                </body>
            </html>
        "#;

    let html = Html::new(input_html, false);
    let result = html.safe();

    assert!(!result.contains(CSS));
  }

  #[test]
  fn test_parse_removes_attributes_starting_with_on() {
    let input_html = r#"
            <html>
                <body>
                    <h1 onclick="alert('hack')">Hello, world!</h1>
                </body>
            </html>
        "#;

    let html = Html::new(input_html, true);
    let result = html.safe();

    assert!(!result.contains("onclick="));
  }

  #[test]
  fn test_starts_with_on_function() {
    assert!(Html::starts_with_on("onclick"));
    assert!(Html::starts_with_on("onload"));
    assert!(!Html::starts_with_on("class"));
    assert!(!Html::starts_with_on("id"));
  }
}
