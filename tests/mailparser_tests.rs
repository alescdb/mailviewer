/* mailparser_tests.rs
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

// TODO: still a lot to do here :)
#[cfg(test)]
mod tests {
  use base64::{engine::general_purpose, Engine};
  use mailviewer::MailParser;
  use std::{cell::OnceCell, error::Error, fs::File, io::Write, path::Path};
  #[test]
  fn sample() -> Result<(), Box<dyn Error>> {
    let temp: OnceCell<&Path> = OnceCell::new();
    let file: String;
    {
      let mut parser = MailParser::new("sample.eml");
      parser.parse()?;
      assert_eq!(parser.from, "John Doe <john@moon.space>");
      assert_eq!(parser.to, "Lucas <lucas@mercure.space>");
      assert_eq!(parser.subject, "Lorem ipsum");
      assert_eq!(parser.date, "2024-10-23 12:27:21");
      assert_eq!(parser.attachments.len(), 1);
      let attachment = &parser.attachments[0];
      assert_eq!(attachment.filename, "Deus_Gnome.png");
      assert_eq!(attachment.content_id, "ii_m2lqbrhv0");
      assert_eq!(attachment.mime_type.as_ref().unwrap(), "image/png");
      file = attachment.write_to_tmp()?;
      temp.set(Path::new(&file)).expect("Failed !");
      assert!(temp.get().unwrap().is_file());
    }
    assert!(temp.get().unwrap().exists() == false);

    Ok(())
  }

  #[test]
  fn test_parse_simple_email() -> Result<(), Box<dyn Error>> {
    // Create a temporary directory
    let dir = std::env::temp_dir();

    // Define a simple email content
    let email_content = r#"From: sender@example.com
To: recipient@example.com
Subject: Test Email
Date: Wed, 27 Sep 2023 14:28:00 +0000
Content-Type: text/plain; charset="UTF-8"

This is a test email body.
"#;

    // Write the email content to a temporary file
    let email_file_path = dir.as_path().join("test_email.eml");
    let mut email_file = File::create(&email_file_path)?;
    email_file.write_all(email_content.as_bytes())?;

    // Initialize the MailParser with the path to the email file
    let mut parser = MailParser::new(email_file_path.to_str().unwrap());
    log::info!("Coucou");

    // Parse the email
    parser.parse()?;

    // Assert that the parsed fields match the expected values
    log::info!("from: {}", parser.from);
    assert_eq!(parser.from, "sender@example.com");
    assert_eq!(parser.to, "recipient@example.com");
    assert_eq!(parser.subject, "Test Email");
    assert_eq!(parser.date, "2023-09-27 14:28:00");
    assert_eq!(parser.body_text, Some("This is a test email body.\n".to_string()));
    assert_eq!(parser.body_html, None);
    assert!(parser.attachments.is_empty());

    Ok(())
  }

  #[test]
  fn test_parse_email_with_html_and_attachments() -> Result<(), Box<dyn Error>> {
    // Create a temporary directory
    let dir = std::env::temp_dir();

    // Define an email content with HTML body and an attachment
    let email_content = r#"From: sender@example.com
To: recipient@example.com
Subject: Test Email with HTML and Attachment
Date: Wed, 27 Sep 2023 14:28:00 +0000
MIME-Version: 1.0
Content-Type: multipart/mixed; boundary="boundary123"

--boundary123
Content-Type: text/html; charset="UTF-8"

<html><body><p>This is a <strong>test</strong> email body.</p><img src="cid:image1"></body></html>
--boundary123
Content-Type: image/png
Content-Transfer-Encoding: base64
Content-ID: <image1>
Content-Disposition: attachment; filename="image.png"

iVBORw0KGgoAAAANSUhEUgAAAAUA
--boundary123--
"#;

    // Write the email content to a temporary file
    let email_file_path = dir.as_path().join("test_email_with_attachment.eml");
    let mut email_file = File::create(&email_file_path)?;
    email_file.write_all(email_content.as_bytes())?;

    // Initialize the MailParser with the path to the email file
    let mut parser = MailParser::new(email_file_path.to_str().unwrap());

    // Parse the email
    parser.parse()?;

    // Assert that the parsed fields match the expected values
    assert_eq!(parser.from, "sender@example.com");
    assert_eq!(parser.to, "recipient@example.com");
    assert_eq!(parser.subject, "Test Email with HTML and Attachment");
    assert_eq!(parser.date, "2023-09-27 14:28:00");

    // Check that the HTML body is parsed
    assert!(parser.body_html.is_some());
    let expected_html = r#"<html><head></head><body><p>This is a <strong>test</strong> email body.</p><img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAUA"></body></html>"#;
    assert_eq!(parser.body_html.as_ref().unwrap(), expected_html);

    // Check that the attachment is parsed
    assert_eq!(parser.attachments.len(), 1);
    let attachment = &parser.attachments[0];
    assert_eq!(attachment.filename, "image.png");
    assert_eq!(attachment.content_id, "image1");
    assert_eq!(attachment.mime_type.as_deref(), Some("image/png"));

    // Check that the attachment body is correct
    let expected_body = general_purpose::STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAUA")?;
    assert_eq!(attachment.body, expected_body);

    Ok(())
  }

  #[test]
  fn test_parse_email_with_multiple_recipients() -> Result<(), Box<dyn Error>> {
    // Create a temporary directory
    let dir = std::env::temp_dir();

    // Define an email content with multiple recipients
    let email_content = r#"From: sender@example.com
To: recipient1@example.com, recipient2@example.com
Cc: cc1@example.com
Bcc: bcc1@example.com
Subject: Test Email with Multiple Recipients
Date: Wed, 27 Sep 2023 14:28:00 +0000
Content-Type: text/plain; charset="UTF-8"

This email has multiple recipients.
"#;

    // Write the email content to a temporary file
    let email_file_path = dir.as_path().join("test_email_multiple_recipients.eml");
    let mut email_file = File::create(&email_file_path)?;
    email_file.write_all(email_content.as_bytes())?;

    // Initialize the MailParser with the path to the email file
    let mut parser = MailParser::new(email_file_path.to_str().unwrap());

    // Parse the email
    parser.parse()?;

    // Assert that the parsed fields match the expected values
    assert_eq!(parser.from, "sender@example.com");
    assert_eq!(parser.to, "recipient1@example.com, recipient2@example.com, cc1@example.com, bcc1@example.com");
    assert_eq!(parser.subject, "Test Email with Multiple Recipients");
    assert_eq!(parser.date, "2023-09-27 14:28:00");
    assert_eq!(parser.body_text, Some("This email has multiple recipients.\n".to_string()));
    assert_eq!(parser.body_html, None);
    assert!(parser.attachments.is_empty());

    Ok(())
  }
}
