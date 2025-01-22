use std::{
  collections::HashMap,
  io::Cursor,
  path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tokio::{
  fs,
  io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
};

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Record {
  pub installed: HashMap<String, RecordItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecordItem {
  pub sha256: String,
  pub last_update_time: String,
  pub files: Vec<PathBuf>,
  pub from: String,
  pub metadata: Option<Metadata>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Metadata {
  pub id: String,
  pub name: String,
  pub description: String,
  pub version: String,
  pub authors: Vec<String>,
  pub url: String,
  pub update: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("IO: {0}")]
  Io(#[from] std::io::Error),
  #[error("serde_json: {0}")]
  SerdeJson(#[from] serde_json::Error),
}

async fn ensure_record(res_mods_dir: &Path) -> Result<(), Error> {
  let seamonkey_file = res_mods_dir.join(".seamonkey");

  if !seamonkey_file.exists() {
    let file = tokio::fs::OpenOptions::new()
      .create_new(true)
      .write(true)
      .open(seamonkey_file)
      .await
      .map_err(Error::Io)?;

    let mut writer = BufWriter::new(file);
    let new_record = serde_json::to_vec(&Record::default()).map_err(Error::SerdeJson)?;
    tokio::io::copy(&mut Cursor::new(new_record), &mut writer)
      .await
      .map_err(Error::Io)?;
  }

  Ok(())
}

pub async fn read_record(res_mods_dir: &Path) -> Result<Record, Error> {
  ensure_record(res_mods_dir).await?;
  let seamonkey_file = res_mods_dir.join(".seamonkey");
  let file = tokio::fs::OpenOptions::new()
    .read(true)
    .open(seamonkey_file)
    .await
    .map_err(Error::Io)?;
  let mut reader = BufReader::new(file);
  let mut buf = Vec::new();
  reader.read_to_end(&mut buf).await.map_err(Error::Io)?;
  serde_json::from_slice(&buf).map_err(Error::SerdeJson)
}

pub async fn write_record(res_mods_dir: &Path, record: &Record) -> Result<(), Error> {
  let file = fs::OpenOptions::new()
    .create(true)
    .truncate(true)
    .write(true)
    .open(res_mods_dir.join(".seamonkey"))
    .await
    .map_err(Error::Io)?;
  let mut writer = BufWriter::new(file);

  tokio::io::copy(
    &mut Cursor::new(serde_json::to_vec(&record).map_err(Error::SerdeJson)?),
    &mut writer,
  )
  .await
  .map_err(Error::Io)?;

  Ok(())
}
