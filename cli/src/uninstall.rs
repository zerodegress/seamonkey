use std::path::Path;

use log::debug;

use crate::record;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("IO: {0}")]
  Io(std::io::Error),
  #[error("Record: {0}")]
  Record(record::Error),
  #[error("Mod not found: {0}")]
  ModNotFound(String),
}

pub async fn uninstall(res_mods_dir: &Path, items: Vec<String>) -> Result<(), Error> {
  debug!("uninstall: {:?}", items);
  let mut record = record::read_record(res_mods_dir)
    .await
    .map_err(Error::Record)?;

  let files_to_uninstall = items
    .iter()
    .map(|item| {
      (
        item,
        record
          .installed
          .get(item)
          .map(|item| item.files.to_owned())
          .or_else(|| {
            record
              .installed
              .iter()
              .find(|(_, record)| {
                record
                  .metadata
                  .as_ref()
                  .is_some_and(|metadata| &metadata.id == item)
              })
              .map(|(_, record)| record.files.to_owned())
          }),
      )
    })
    .map(|(item, x)| x.ok_or(Error::ModNotFound(item.to_owned())))
    .collect::<Result<Vec<_>, _>>()?
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

  for item in items.iter() {
    record.installed.remove(item);
  }

  record::write_record(res_mods_dir, &record)
    .await
    .map_err(Error::Record)?;

  for file in files_to_uninstall {
    tokio::fs::remove_file(res_mods_dir.join(file))
      .await
      .map_err(Error::Io)?;
  }

  Ok(())
}
