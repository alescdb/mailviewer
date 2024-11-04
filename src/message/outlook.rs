use std::{error::Error, path::PathBuf};

use base64::{engine::general_purpose, Engine};
use msg_parser::Outlook;
use uuid::Uuid;

use super::{attachment::Attachment, message::Message};

#[derive(Debug, Default, Clone)]
pub struct OutlookMessage {
  file: String,
  temp: PathBuf,
  pub from: String,
  pub to: String,
  pub date: String,
  pub subject: String,
  pub body: Option<String>,
  pub attachments: Vec<Attachment>,
}

impl OutlookMessage {
  pub fn new(file: &str) -> Self {
    Self {
      file: file.to_string(),
      from: String::new(),
      temp: PathBuf::new(),
      to: String::new(),
      date: String::new(),
      subject: String::new(),
      body: None,
      attachments: vec![],
    }
  }

  fn person_to_string(person: &msg_parser::Person) -> String {
    format!("{} <{}>", person.name, person.email)
  }

  fn person_list_to_string(persons: &Vec<msg_parser::Person>) -> String {
    persons
      .iter()
      .map(|person| OutlookMessage::person_to_string(person))
      .collect::<Vec<String>>()
      .join(", ")
  }

  fn parse(&mut self) -> Result<(), Box<dyn Error>> {
    let outlook = Outlook::from_path("tests/sample.msg").unwrap();
    self.from = OutlookMessage::person_to_string(&outlook.sender);
    self.to = OutlookMessage::person_list_to_string(&outlook.to);
    self.subject = outlook.subject;
    self.date = outlook.headers.date;
    self.body = Some(outlook.body.clone());
    println!("body_html => {:?}", &outlook.body.clone());
    println!("content_type => {:?}", outlook.headers.content_type);
    println!("message_id => {:?}", outlook.headers.message_id);
    println!("reply_to => {:?}", outlook.headers.reply_to);
    println!(
      "attachments.capacity => {:?}",
      outlook.attachments.capacity()
    );

    for i in 0..outlook.attachments.capacity() {
      let att = &outlook.attachments[i];
      println!("att.file_name => {:?}", att.file_name);
      println!("att.display_name => {:?}", att.display_name);
      println!("att.extension => {:?}", att.extension);
      println!("att.mime_tag => {:?}", att.mime_tag);
      self.attachments.push(Attachment {
        temp: String::new(),
        filename: att.file_name.clone(),
        content_id: Uuid::new_v4().simple().to_string(),
        body: match general_purpose::STANDARD.decode(&att.payload) {
          Ok(body) => body,
          Err(err) => {
            println!("Failed to decode body : {}", err);
            vec![]
          }
        },
        mime_type: Some(att.mime_tag.clone()),
      });
    }
    let att = outlook.attachments.first().unwrap();
    println!("attachments.capacity => {:?}", outlook.attachments.first());
    println!("att.file_name => {:?}", att.file_name);
    println!("att.display_name => {:?}", att.display_name);
    println!("att.extension => {:?}", att.extension);
    println!("att.mime_tag => {:?}", att.mime_tag);

    Ok(())
  }
}

impl Message for OutlookMessage {
  fn from(&self) -> String {
    self.from.clone()
  }

  fn to(&self) -> String {
    self.to.clone()
  }

  fn subject(&self) -> String {
    self.subject.clone()
  }

  fn date(&self) -> String {
    self.date.clone()
  }

  fn attachments(&self) -> Vec<Attachment> {
    self.attachments.clone()
  }

  fn body_html(&self) -> Option<String> {
    None
  }

  fn body_text(&self) -> Option<String> {
    self.body.clone()
  }
}

#[cfg(test)]
mod tests {
  use crate::message::outlook::OutlookMessage;
  use std::error::Error;

  #[test]
  fn test_outlook() -> Result<(), Box<dyn Error>> {
    let mut parser = OutlookMessage::new("sample.eml");
    parser.parse()?;
<<<<<<< HEAD
    assert_eq!(parser.from, "");
    assert_eq!(
      parser.to,
      ""
    );
    assert_eq!(
      parser.subject,
      ""
=======
    assert_eq!(parser.from, "");
    assert_eq!(
      parser.to,
      ""
    );
    assert_eq!(
      parser.subject,
      ""
>>>>>>> e6e2115093b98f5e8b9a29fefc64231108fc5979
    );
    assert_eq!(parser.date, "Fri, 25 May 2018 10:31:04 +0000");
    assert_eq!(parser.attachments.len(), 1);
    assert_eq!(parser.attachments[0].filename, "image001.jpg");
    assert_eq!(
      parser.attachments[0].mime_type.clone().unwrap(),
      "image/jpeg"
    );

    Ok(())
  }
}
