use crate::message::message::MessageParser;
use msg_parser::Outlook;
use std::error::Error;

use super::{attachment::Attachment, message::Message};

#[derive(Debug, Default, Clone)]
pub struct OutlookMessage {
  file: String,
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
}

impl Message for OutlookMessage {
  fn parse(&mut self) -> Result<(), Box<dyn Error>> {
    let outlook = Outlook::from_path(&self.file).unwrap();
    self.from = OutlookMessage::person_to_string(&outlook.sender);
    self.to = OutlookMessage::person_list_to_string(&outlook.to);
    self.subject = outlook.subject;
    self.date = outlook.headers.date;
    self.body = Some(outlook.body.clone());

    for i in 0..outlook.attachments.capacity() {
      let att = &outlook.attachments[i];
      self.attachments.push(Attachment {
        filename: att.file_name.clone(),
        content_id: att.file_name.clone(), // Uuid::new_v4().simple().to_string(),
        body: match hex::decode(&att.payload) {
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
    println!(
      "attachments.capacity => {:?}",
      outlook.attachments.capacity()
    );
    println!("att.file_name => {:?}", att.file_name);
    println!("att.display_name => {:?}", att.display_name);
    println!("att.extension => {:?}", att.extension);
    println!("att.mime_tag => {:?}", att.mime_tag);

    Ok(())
  }

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

impl Drop for OutlookMessage {
  fn drop(&mut self) {
    log::warn!("OutlookMessage::drop()");
    MessageParser::cleanup();
  }
}

#[cfg(test)]
mod tests {
  use crate::message::{message::Message, outlook::OutlookMessage};
  use std::error::Error;

  #[test]
  fn test_outlook() -> Result<(), Box<dyn Error>> {
    let mut parser = OutlookMessage::new("sample.msg");
    parser.parse()?;
    assert_eq!(parser.from, "John Doe <john@moon.space>");
    assert_eq!(parser.to, "Lucas <lucas@mercure.space>");
    assert_eq!(parser.subject, "Lorem ipsum");
    assert_eq!(parser.date, "");
    assert_eq!(parser.attachments.len(), 3);
    assert_eq!(parser.attachments[0].filename, "image001.png");
    assert!(parser.body.clone().unwrap().contains("Hello Lucas"));
    assert_eq!(
      parser.attachments[0].mime_type.clone().unwrap(),
      "image/png"
    );
    Ok(())
  }
}
