use std::path::Path;

use temp_dir::TempDir;

use crate::{install, record};

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("Record: {0}")]
  Record(record::Error),
  #[error("Install: {0}")]
  Install(install::Error),
}

pub async fn update(
  res_mods_dir: &Path,
  temp_dir: &TempDir,
  yes_for_all: bool,
) -> Result<(), Error> {
  let record = record::read_record(res_mods_dir)
    .await
    .map_err(Error::Record)?;
  let update_items = record
    .installed
    .iter()
    .filter_map(|(_, x)| x.metadata.to_owned())
    .map(|x| x.update)
    .collect::<Vec<_>>();
  install::install(res_mods_dir, update_items, temp_dir, yes_for_all)
    .await
    .map_err(Error::Install)?;
  Ok(())
}
