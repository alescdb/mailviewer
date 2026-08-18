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
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use base64::engine::general_purpose;
use base64::Engine;

use crate::message::attachment::Attachment;

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
  inline_images: HashMap<String, String>,
}

impl Html {
  pub fn new(body: &str, strip_css: bool) -> Self {
    Self {
      body: body.to_string(),
      strip_css,
      allow_remote: false,
      inline_images: HashMap::new(),
    }
  }

  /// Whether the message may pull content from the network, i.e. the state of
  /// the "Show remote images" button.
  pub fn allow_remote(mut self, allow_remote: bool) -> Self {
    self.allow_remote = allow_remote;
    self
  }

  /// The attachments a `cid:` source can point at.
  pub fn inline_images(mut self, attachments: &[Attachment]) -> Self {
    self.inline_images = attachments
      .iter()
      .filter(|attachment| !attachment.content_id.is_empty())
      .filter_map(|attachment| {
        let mime_type = attachment.mime_type.as_deref()?;
        Some((
          attachment.content_id.clone(),
          format!(
            "data:{};base64,{}",
            mime_type,
            general_purpose::STANDARD.encode(&attachment.body)
          ),
        ))
      })
      .collect();
    self
  }

  pub fn escape(value: &str) -> String {
    ammonia::clean_text(value)
  }

  pub fn policy(&self) -> &str {
    if self.allow_remote {
      CSP_ALLOW_REMOTE
    } else {
      CSP_BLOCK_REMOTE
    }
  }

  pub fn safe(&self) -> String {
    let policy = self.policy();

    // The document is built here rather than taken from the message, so the
    // head is ours and nothing of the message is parsed before the policy.
    format!(
      concat!(
        "<!doctype html><html><head>",
        "<meta charset=\"utf-8\">",
        "<meta http-equiv=\"Content-Security-Policy\" content=\"{}\">",
        "{}</head><body>{}</body></html>"
      ),
      policy,
      if self.strip_css { CSS } else { "" },
      self.clean()
    )
  }

  fn clean(&self) -> String {
    let mut builder = ammonia::Builder::default();

    // cid: is not rendered, it only has to survive long enough for the filter
    // below to turn it into the data: uri of the attachment it points at.
    let mut schemes: HashSet<&str> = ammonia::Builder::default().clone_url_schemes();
    schemes.insert("data");
    schemes.insert("cid");
    builder.url_schemes(schemes);

    if !self.strip_css {
      // A <style> of the message is kept as it is : it cannot script, and the
      // policy above is what keeps it from reaching the network.
      let mut tags: HashSet<&str> = ammonia::Builder::default().clone_tags();
      tags.insert("style");
      let mut content: HashSet<&str> = ammonia::Builder::default().clone_clean_content_tags();
      content.remove("style");
      let mut attributes: HashSet<&str> = ammonia::Builder::default().clone_generic_attributes();
      attributes.insert("style");
      attributes.insert("class");

      builder
        .tags(tags)
        .clean_content_tags(content)
        .generic_attributes(attributes);
    }

    let inline_images = self.inline_images.clone();
    builder.attribute_filter(move |element, attribute, value| {
      if element == "img" && attribute == "src" {
        if let Some(content_id) = value.strip_prefix("cid:") {
          return inline_images
            .get(content_id)
            .map(|uri| Cow::Owned(uri.clone()));
        }
      }
      Some(Cow::Borrowed(value))
    });

    builder.clean(&self.body).to_string()
  }

  /// The `<li>` list of attachments for the printed page. Both the file name and
  /// the mime type come from the message, so both are escaped.
  fn print_attachment_list(attachments: &[Attachment]) -> String {
    attachments
      .iter()
      .map(|attachment| {
        let filename = Html::escape(&attachment.filename);
        match attachment.mime_type.as_deref() {
          Some(mime_type) if !mime_type.is_empty() => {
            format!("<li>{filename} ({})</li>", Html::escape(mime_type))
          }
          _ => format!("<li>{filename}</li>"),
        }
      })
      .collect::<Vec<_>>()
      .join("\n")
  }

  pub fn safe_print(
    &self,
    from: &str,
    to: &str,
    date: &str,
    subject: &str,
    attachments: &[Attachment],
  ) -> String {
    let from = Self::escape(from);
    let to = Self::escape(to);
    let date = Self::escape(date);
    let subject = Self::escape(subject);
    let policy = self.policy();
    let content = self.clean().to_string();
    let attachments = Self::print_attachment_list(attachments);

    format!(
      r#"<!doctype html>
      <html>
      <head>
        <meta charset="utf-8">
        <meta http-equiv="Content-Security-Policy" content="{policy}">
        <style>
        pre {{
            white-space: pre-wrap;
            overflow-wrap: anywhere;
            word-break: break-word;
        }}
        th {{
            vertical-align: top;
            text-align: left;
        }}
        </style>
      </head>
      <body>
        <table class="header">
          <tr><th>From:&nbsp;</th><td>{from}</td></tr>
          <tr><th>To:&nbsp;</th><td>{to}</td></tr>
          <tr><th>Date:&nbsp;</th><td>{date}</td></tr>
          <tr><th>Subject:&nbsp;</th><td>{subject}</td></tr>
        </table>
        <hr />
        <div class="body">
          {content}
        </div>
        <hr />
        <ul>
          {attachments}
        </ul>
      </body>
      </html>"#
    )
  }
}

#[cfg(test)]
mod tests {
  use std::error::Error;
  use std::fs;

  use crate::html::Html;
  use crate::message::message::{Message, MessageParser};
  use crate::{gio, utils};

  const SHELL: &str = "<!doctype html><html><head><meta charset=\"utf-8\"><meta http-equiv=\"Content-Security-Policy\" content=\"";

  #[test]
  fn html() -> Result<(), Box<dyn Error>> {
    let body = Html::new(&fs::read_to_string("tests/test.html")?, true)
      .safe()
      .to_lowercase();

    assert!(!body.contains("onblur="));
    assert!(!body.contains("onclick="));
    assert!(!body.contains("onchange="));
    // forcing the css drops what the message brought
    assert!(!body.contains("style=\"color"));
    assert!(!body.contains("class="));

    assert!(!body.contains("<script"));
    assert!(!body.contains("console.log"));
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
  fn the_document_is_ours() {
    let body = Html::new("<p>hello</p>", false).safe();

    assert!(body.starts_with(SHELL), "unexpected shell: {body}");
    assert!(body.ends_with("<p>hello</p></body></html>"));
  }

  #[test]
  fn the_head_of_the_message_is_dropped() {
    // A '>' in an attribute of the head used to swallow the policy back when it
    // was inserted into the html of the message.
    let body = Html::new(
      "<html><head title=\">\"><title>t</title></head><body>hi</body></html>",
      false,
    )
    .safe();

    assert!(body.starts_with(SHELL));
    assert!(!body.contains("title=\">\""));
    assert_eq!(body.matches("Content-Security-Policy").count(), 1);
  }

  #[test]
  fn csp_blocks_remote_content_by_default() {
    let body = Html::new("<p>hello</p>", false).safe();

    assert!(body.contains("img-src data:;"));
    assert!(!body.contains("font-src"));
  }

  #[test]
  fn csp_opens_up_when_remote_content_is_allowed() {
    let body = Html::new("<p>hello</p>", false).allow_remote(true).safe();

    assert!(body.contains("img-src data: http: https:;"));
    assert!(body.contains("font-src http: https:;"));
  }

  #[test]
  fn a_style_of_the_message_is_kept_as_it_is() {
    let message = "<style>td > p { color: red; }</style><p>hi</p>";

    let body = Html::new(message, false).safe();
    assert!(body.contains("<style>td > p { color: red; }</style>"));

    // ... unless the css is forced
    let body = Html::new(message, true).safe();
    assert!(!body.contains("color: red"));
  }

  #[test]
  fn tags_that_are_not_expected_in_a_message_are_dropped() {
    let body = Html::new(
      "<svg><use href=\"http://x/y\"/></svg><base href=\"http://evil/\"><template><img src=\"http://x\"></template>",
      false,
    )
    .safe();

    assert!(!body.contains("<svg"));
    assert!(!body.contains("<base"));
    assert!(!body.contains("<template"));
    assert!(!body.contains("http://"));
  }

  #[test]
  fn a_javascript_link_does_not_survive() {
    let body = Html::new("<a href=\"javascript:alert(1)\">x</a>", false).safe();

    assert!(!body.contains("javascript:"));
  }

  #[test]
  fn an_inline_image_becomes_a_data_uri() {
    let file = gio::File::for_path("sample.eml");

    utils::spawn_and_wait_new_ctx(async move {
      let mut message = MessageParser::new(&file, None).await.expect("File opened");
      message.parse(None).unwrap();

      let body = Html::new(&message.body_html().unwrap(), false)
        .inline_images(&message.attachments())
        .safe();

      assert!(!body.contains("cid:"), "a cid: source was left behind");
      assert!(body.contains("<img src=\"data:image/png;base64,iVBOR"));
    });
  }

  #[test]
  fn an_inline_image_without_its_attachment_is_dropped() {
    let body = Html::new("<img src=\"cid:nothing\">", false).safe();

    assert!(!body.contains("cid:"));
    assert!(!body.contains("src="));
  }

  fn attachment(filename: &str, mime_type: Option<&str>) -> crate::message::attachment::Attachment {
    crate::message::attachment::Attachment {
      filename: filename.to_string(),
      content_id: String::new(),
      body: vec![],
      mime_type: mime_type.map(String::from),
    }
  }

  #[test]
  fn print_attachment_list_escapes_both_fields() {
    let list = Html::print_attachment_list(&[
      attachment("Deus_Gnome.png", Some("image/png")),
      attachment("a&b<c>.txt", Some("text/plain")),
      attachment("report.pdf", Some("application/pdf\"><img src=x>")),
      attachment("no-mime.bin", None),
      attachment("empty-mime.bin", Some("")),
    ]);

    assert_eq!(
      list,
      "<li>Deus_Gnome.png (image&#47;png)</li>\n\
       <li>a&amp;b&lt;c&gt;.txt (text&#47;plain)</li>\n\
       <li>report.pdf (application&#47;pdf&quot;&gt;&lt;img&#32;src&#61;x&gt;)</li>\n\
       <li>no-mime.bin</li>\n\
       <li>empty-mime.bin</li>"
    );
  }
}
