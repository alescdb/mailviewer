use std::{error::Error, fmt, fs};

#[derive(Debug, Clone)]
pub struct Attachment {
  pub temp: String,
  pub filename: String,
  pub content_id: String,
  pub body: Vec<u8>,
  pub mime_type: Option<String>,
}

impl Attachment {
  pub fn write_to_tmp(&self) -> Result<String, Box<dyn Error>> {
    self.write_to_file(&self.temp)?;
    Ok(self.temp.to_string())
  }

  pub fn write_to_file(&self, file: &str) -> std::io::Result<()> {
    fs::write(&file, &self.body)
  }
}

impl fmt::Display for Attachment {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(
      f,
      "Attachment(content_id: {}, filename: {}, mime_type: {})",
      self.content_id,
      self.filename,
      self.mime_type.as_deref().unwrap_or("None")
    )
  }
}