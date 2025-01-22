use std::path::{Path, PathBuf};

use async_zip::error::ZipError;
use futures_lite::AsyncReadExt;
use log::debug;
use tokio::{
  fs,
  io::{AsyncBufRead, AsyncSeek, BufReader},
};
use tokio_util::compat::TokioAsyncWriteCompatExt;
use url::Url;
use uuid::Uuid;

use crate::record;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("No Mod to install")]
  NoModToInstall,
  #[error("Unknown URL scheme")]
  UnknownUrlScheme(String),
  #[error("Record：{0}")]
  Record(record::Error),
  #[error("Zip：{0}")]
  Zip(ZipError),
  #[error("IO: {0}")]
  Io(std::io::Error),
  #[error("TOML：{0}")]
  DeToml(toml::de::Error),
  #[error("File conflict：{0}")]
  FileConflict(PathBuf, Vec<FileConfilctCheck>),
  #[error("Mod not found：{0}")]
  ModNotFound(Url),
}

#[derive(Debug)]
pub struct FileConfilctCheck {
  pub installed: String,
  pub metadata: Option<record::Metadata>,
}

pub async fn install(res_mods_dir: &Path, items: Vec<String>) -> Result<(), Error> {
  debug!("install: {:?}", items);
  if items.is_empty() {
    Err(Error::NoModToInstall)
  } else {
    for item in items.iter() {
      if let Ok(url) = item.parse::<Url>() {
        match url.scheme() {
          "file" => {
            install_from_file(res_mods_dir, PathBuf::from(url.path()).as_ref()).await?;
          }
          scheme => return Err(Error::UnknownUrlScheme(scheme.to_owned())),
        }
      } else {
        install_from_file(res_mods_dir, PathBuf::from(item).as_ref()).await?;
      }
    }
    Ok(())
  }
}

async fn install_from_file(res_mods_dir: &Path, mod_to_install: &Path) -> Result<(), Error> {
  let from_url = Url::from_file_path(if mod_to_install.is_absolute() {
    mod_to_install.to_string_lossy().to_string()
  } else {
    std::env::current_dir()
      .expect("wtf current dir")
      .join(mod_to_install)
      .to_string_lossy()
      .to_string()
  })
  .expect("wtf file url");

  if !fs::try_exists(&mod_to_install).await.map_err(Error::Io)? {
    return Err(Error::ModNotFound(from_url.to_owned()));
  }

  let sha256 = sha256::try_async_digest(mod_to_install)
    .await
    .map_err(Error::Io)?;

  install_zip(
    res_mods_dir,
    BufReader::new(fs::File::open(mod_to_install).await.map_err(Error::Io)?),
    from_url,
    sha256,
    Uuid::new_v4().to_string(),
  )
  .await
}

async fn install_zip(
  res_mods_dir: &Path,
  mod_to_install: impl AsyncBufRead + AsyncSeek + Unpin,
  from_url: Url,
  sha256: String,
  install_id: String,
) -> Result<(), Error> {
  let mut record = record::read_record(res_mods_dir)
    .await
    .map_err(Error::Record)?;

  let mut mod_to_install_zip =
    async_zip::tokio::read::seek::ZipFileReader::with_tokio(mod_to_install)
      .await
      .map_err(Error::Zip)?;

  let record_item = record::RecordItem {
    sha256,
    last_update_time: chrono::Local::now().to_string(),
    from: from_url.to_string(),
    files: mod_to_install_zip
      .file()
      .entries()
      .iter()
      .filter_map(|x| {
        match x.dir().map(|x_is_dir| {
          if x_is_dir {
            None
          } else {
            Some(x.filename().to_owned().into_string().map(PathBuf::from))
          }
        }) {
          Ok(Some(x)) => Some(x),
          Ok(None) => None,
          Err(err) => Some(Err(err)),
        }
      })
      .collect::<Result<Vec<_>, _>>()
      .map_err(Error::Zip)?,
    metadata: {
      if let Some((index, _)) =
        mod_to_install_zip
          .file()
          .entries()
          .iter()
          .enumerate()
          .find(|(_, file)| {
            file
              .filename()
              .as_str()
              .map(|filename| filename == "seamonkey.toml")
              .unwrap_or(false)
          })
      {
        Some({
          let mut reader = mod_to_install_zip
            .reader_without_entry(index)
            .await
            .map_err(Error::Zip)?;
          let mut buf = String::new();
          reader.read_to_string(&mut buf).await.map_err(Error::Io)?;
          toml::from_str(buf.as_str()).map_err(Error::DeToml)?
        })
      } else {
        None
      }
    },
  };

  for file in mod_to_install_zip.file().entries().iter() {
    let file_path = sanitize_file_path(file.filename().as_str().map_err(Error::Zip)?);
    let target_path = res_mods_dir.join(&file_path);

    if fs::try_exists(&target_path).await.map_err(Error::Io)? {
      if file.dir().map_err(Error::Zip)? {
        continue;
      }
      let check_list = record
        .installed
        .iter()
        .filter(|(_, record)| {
          record
            .files
            .iter()
            .any(|record_file_path| record_file_path == &file_path)
        })
        .map(|(installed, record)| FileConfilctCheck {
          installed: installed.to_owned(),
          metadata: record.metadata.to_owned(),
        })
        .collect::<Vec<_>>();
      return Err(Error::FileConflict(file_path, check_list));
    }
  }

  record.installed.insert(install_id, record_item);

  for (index, file) in mod_to_install_zip
    .file()
    .entries()
    .to_vec()
    .iter()
    .enumerate()
  {
    if file.dir().map_err(Error::Zip)? {
      let dir_path = sanitize_file_path(file.filename().as_str().map_err(Error::Zip)?);
      let target_path = res_mods_dir.join(&dir_path);
      tokio::fs::create_dir_all(&target_path)
        .await
        .map_err(Error::Io)?;
    } else {
      let mut reader = mod_to_install_zip
        .reader_without_entry(index)
        .await
        .map_err(Error::Zip)?;
      let file_path = sanitize_file_path(file.filename().as_str().map_err(Error::Zip)?);
      let target_path = res_mods_dir.join(&file_path);
      let mut writer = BufReader::new(
        tokio::fs::OpenOptions::new()
          .create_new(true)
          .write(true)
          .open(&target_path)
          .await
          .map_err(Error::Io)?,
      )
      .compat_write();
      futures_lite::io::copy(&mut reader, &mut writer)
        .await
        .map_err(Error::Io)?;
    }
  }

  record::write_record(res_mods_dir, &record)
    .await
    .map_err(Error::Record)?;

  Ok(())
}

fn sanitize_file_path(path: impl AsRef<str>) -> PathBuf {
  // Replaces backwards slashes
  path
    .as_ref()
    .replace('\\', "/")
    // Sanitizes each component
    .split('/')
    .map(sanitize_filename::sanitize)
    .collect()
}
