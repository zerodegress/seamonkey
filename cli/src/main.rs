use std::{path::PathBuf, process::exit};

use clap::Parser;
use log::{debug, error};
use tokio::fs;

mod cli;
mod install;
mod record;
mod uninstall;

#[tokio::main]
async fn main() {
  env_logger::init();
  let cli = cli::Cli::parse();
  run_with_handle_error(cli).await;
}

#[derive(Debug, thiserror::Error)]
enum Error {
  #[error("Game dir not provided")]
  GameDirNotProvided,
  #[error("Incorrect game directory structure")]
  IncorrectGameDirectoryStructure,
  #[error("IO: {0}")]
  Io(std::io::Error),
  #[error("Install：{0}")]
  Install(install::Error),
  #[error("Uninstall: {0}")]
  Uninstall(uninstall::Error),
}

fn print_error(err: &Error) {
  error!("{:?}", err);
  match err {
    Error::GameDirNotProvided => {
      println!("未提供游戏目录");
    }
    Error::Io(err) => {
      println!("IO错误：{}", err);
    }
    Error::IncorrectGameDirectoryStructure => {
      println!("游戏目录结构错误");
    }
    Error::Install(err) => match err {
      install::Error::Io(err) => {
        println!("安装时发生IO错误：{}", err);
      }
      install::Error::Zip(err) => {
        println!("安装时访问压缩包出错：{}", err);
      }
      install::Error::UnknownUrlScheme(scheme) => {
        println!("未知的URL方案：{}", scheme);
      }
      install::Error::NoModToInstall => {
        println!("未指定要安装的Mod")
      }
      install::Error::ModNotFound(url) => {
        println!("指定的Mod未找到：{}", url);
      }
      install::Error::FileConflict(file_path, check_list) => {
        println!("要安装的Mod与已有的Mod发生文件冲突：{:?}", file_path);
        for check in check_list {
          if let Some(metadata) = &check.metadata {
            println!(
              "  - {}({}), 来自{}",
              metadata.name, metadata.id, metadata.url
            );
          } else {
            println!("  - {}, 元数据未找到", check.installed);
          }
        }
      }
      install::Error::DeToml(err) => {
        println!("解析Mod元数据出错: {}", err);
      }
      install::Error::Record(err) => match err {
        record::Error::Io(err) => {
          println!("读取安装记录发生IO错误：{}", err);
        }
        record::Error::SerdeJson(err) => {
          println!("解析安装记录出错：{}", err);
        }
      },
      install::Error::UserInterrupt => {
        println!("安装已被用户取消");
      }
    },
    Error::Uninstall(err) => match err {
      uninstall::Error::Io(err) => {
        println!("卸载时IO发生错误: {}", err);
      }
      uninstall::Error::ModNotFound(not_found) => {
        println!("未找到要卸载的Mod：{}", not_found);
      }
      uninstall::Error::Record(err) => {
        println!("卸载时访问安装记录发生错误：{}", err);
      }
    },
  }
}

async fn run_with_handle_error(cli: cli::Cli) {
  for _ in 0..=3 {
    if let Err(err) = run(&cli).await {
      match &err {
        Error::Install(install::Error::FileConflict(_, check_list)) => {
          print_error(&err);
          println!("是否卸载冲突的所有Mod？[y/N]");
          let mut buf = String::new();
          match std::io::stdin().read_line(&mut buf).map_err(Error::Io) {
            Ok(_) => {
              if buf.starts_with("Y") || buf.starts_with("y") {
                if let Err(err) = {
                  let res_mods_dir = find_res_mods_dir(&cli.game_dir).await;
                  match res_mods_dir {
                    Err(err) => Err(err),
                    Ok(res_mods_dir) => uninstall::uninstall(
                      &res_mods_dir,
                      check_list
                        .iter()
                        .map(|check| check.installed.to_owned())
                        .collect(),
                    )
                    .await
                    .map_err(Error::Uninstall),
                  }
                } {
                  print_error(&err);
                } else {
                  continue;
                }
              }
            }
            Err(err) => print_error(&err),
          }
        }
        err => print_error(err),
      }
      exit(-1);
    } else {
      break;
    }
  }
}

async fn run(cli: &cli::Cli) -> Result<(), Error> {
  let res_mods_dir = find_res_mods_dir(&cli.game_dir).await?;

  match &cli.subcommand {
    cli::SubCommand::Install { items } => install::install(res_mods_dir.as_ref(), items.to_owned())
      .await
      .map_err(Error::Install),
    cli::SubCommand::Uninstall { items } => {
      uninstall::uninstall(res_mods_dir.as_ref(), items.to_owned())
        .await
        .map_err(Error::Uninstall)
    }
  }
}

async fn find_res_mods_dir(game_dir: &Option<PathBuf>) -> Result<PathBuf, Error> {
  let game_dir = game_dir.to_owned().ok_or(Error::GameDirNotProvided)?;
  let bin_dir = game_dir.join("bin");

  debug!("bin_dir: {:?}", bin_dir);

  if !fs::try_exists(bin_dir.as_path()).await.map_err(Error::Io)? {
    Err(Error::IncorrectGameDirectoryStructure)
  } else {
    let dir_names = {
      let mut read_dir = tokio::fs::read_dir(bin_dir.as_path())
        .await
        .map_err(Error::Io)?;
      let mut dir_names = Vec::new();
      while let Some(dir) = read_dir.next_entry().await.map_err(Error::Io)? {
        dir_names.push(dir.file_name().to_owned());
      }
      dir_names
    };

    debug!("versions: {:?}", dir_names);

    let max_version = dir_names
      .into_iter()
      .map(|dir_name| dir_name.to_string_lossy().parse::<u64>())
      .collect::<Result<Vec<_>, _>>()
      .map_err(|_| Error::IncorrectGameDirectoryStructure)?
      .into_iter()
      .max()
      .map(|x| format!("{}", x))
      .ok_or(Error::IncorrectGameDirectoryStructure)?;

    Ok(bin_dir.join(max_version).join("res_mods"))
  }
}
