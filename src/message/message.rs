use crate::message::{electronicmail::ElectronicMail, outlook::OutlookMessage};
use super::attachment::Attachment;

pub trait Message {
  fn from(&self) -> String;
  fn to(&self) -> String;
  fn subject(&self) -> String;
  fn date(&self) -> String;
  fn attachments(&self) -> Vec<Attachment>;
  fn body_html(&self) -> Option<String>;
  fn body_text(&self) -> Option<String>;

}

struct MessageData;
impl MessageData {
  fn new(file: &str) -> Box<dyn Message> where Self: Sized {
    log::debug!("Message::new({})", file);
    if file.ends_with(".eml") {
      let mut eml = ElectronicMail::new(file);
      eml.parse().unwrap();
      return Box::new(eml);
    }
    if file.ends_with(".msg") {
      return Box::new(OutlookMessage::new(file));
    }
    panic!("File {} has unsupported extension", file);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_sample() {
    let message = MessageData::new("sample.eml");
    assert_eq!(message.from(), "John Doe <john@moon.space>");
    assert_eq!(message.to(), "Lucas <lucas@mercure.space>");
    assert_eq!(message.subject(), "Lorem ipsum");
    assert_eq!(message.date(), "2024-10-23 12:27:21");
    assert_eq!(message.attachments().len(), 1);
    let attachment = &message.attachments()[0];
    assert_eq!(attachment.filename, "Deus_Gnome.png");
    assert_eq!(attachment.content_id, "ii_m2lqbrhv0");
    assert_eq!(attachment.mime_type.as_ref().unwrap(), "image/png");
  }
} 
