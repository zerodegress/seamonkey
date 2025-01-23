use std::cell::RefCell;
use std::collections::VecDeque;
use std::env::current_dir;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::time::Duration;

use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow};
use gtk4 as gtk;

#[derive(Debug, Clone)]
enum Event {
  LogUpdate(String),
}

fn main() -> glib::ExitCode {
  env_logger::init();

  let application = Application::builder()
    .application_id("ink.zerodgress.seamonkey.gtk")
    .build();

  application.connect_activate(|app| {
    let event_queue = RefCell::new(VecDeque::<Event>::new());
    let mods_to_install = RefCell::new(Vec::<String>::new());
    let seamonkey_cli_path = RefCell::new(current_dir().expect("wtf current_dir").join(format!(
      "seamonkey_cli{}",
      if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        ""
      } else if cfg!(target_os = "windows") {
        ".exe"
      } else {
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "linux")))]
        compile_error!("This program only supports Windows and Linux!");
        ""
      }
    )));
    let game_dir_path = RefCell::new(current_dir().expect("wtf current_dir"));

    let window = ApplicationWindow::builder()
      .application(app)
      .title("First GTK Program")
      .default_width(350)
      .default_height(70)
      .build();

    let vbox = gtk::Box::new(gtk4::Orientation::Vertical, 10);
    vbox.set_margin_bottom(10);
    vbox.set_margin_top(10);
    vbox.set_margin_start(10);
    vbox.set_margin_end(10);
    window.set_child(Some(&vbox));

    let seamonkey_cli_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    {
      let seamonkey_cli_entry = gtk::Entry::new();
      seamonkey_cli_entry.set_placeholder_text(Some("seamonkey_cli位置"));
      seamonkey_cli_entry.set_text(
        seamonkey_cli_path
          .borrow()
          .to_string_lossy()
          .to_string()
          .as_str(),
      );
      seamonkey_cli_entry.set_hexpand(true);
      seamonkey_cli_entry.set_editable(false);
      seamonkey_cli_box.append(&seamonkey_cli_entry);

      let seamonkey_cli_choose_btn = gtk::Button::with_label("打开");
      seamonkey_cli_choose_btn.connect_clicked({
        let window = window.to_owned();
        let seamonkey_cli_entry = seamonkey_cli_entry.to_owned();
        let seamonkey_cli_path = seamonkey_cli_path.to_owned();
        move |_| {
          let file_chooser = gtk::FileChooserDialog::new(
            Some("选择游戏根目录"),
            Some(&window),
            gtk::FileChooserAction::Open,
            &[
              ("取消", gtk::ResponseType::Cancel),
              ("打开", gtk::ResponseType::Accept),
            ],
          );

          file_chooser.run_async({
            let seamonkey_cli_entry = seamonkey_cli_entry.to_owned();
            let seamonkey_cli_path = seamonkey_cli_path.to_owned();
            move |file_chooser, _| {
              if let Some(file) = file_chooser.file() {
                if let Some(file_path) = file.path() {
                  let file_path_string = file_path.to_string_lossy().to_string();
                  seamonkey_cli_entry.set_text(&file_path_string);
                  let mut seamonkey_cli_path = seamonkey_cli_path.borrow_mut();
                  *seamonkey_cli_path = file_path;
                }
              }
              file_chooser.close();
            }
          });
        }
      });
      seamonkey_cli_box.append(&seamonkey_cli_choose_btn);
    }
    vbox.append(&seamonkey_cli_box);

    let game_dir_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    {
      let game_dir_entry = gtk::Entry::new();
      game_dir_entry.set_text(
        game_dir_path
          .borrow()
          .to_string_lossy()
          .to_string()
          .as_str(),
      );
      game_dir_entry.set_hexpand(true);
      game_dir_entry.set_editable(false);
      game_dir_box.append(&game_dir_entry);

      let game_dir_choose_btn = gtk::Button::with_label("打开");
      game_dir_choose_btn.connect_clicked({
        let window = window.to_owned();
        let game_dir_entry = game_dir_entry.to_owned();
        let game_dir_path = game_dir_path.to_owned();
        move |_| {
          let file_chooser = gtk::FileChooserDialog::new(
            Some("选择游戏根目录"),
            Some(&window),
            gtk::FileChooserAction::SelectFolder,
            &[
              ("取消", gtk::ResponseType::Cancel),
              ("打开", gtk::ResponseType::Accept),
            ],
          );

          file_chooser.run_async({
            let game_dir_entry = game_dir_entry.to_owned();
            let game_dir_path = game_dir_path.to_owned();
            move |file_chooser, _| {
              if let Some(file) = file_chooser.file() {
                if let Some(file_path) = file.path() {
                  let file_path_string = file_path.to_string_lossy().to_string();
                  game_dir_entry.set_text(&file_path_string);
                  let mut game_dir_path = game_dir_path.borrow_mut();
                  *game_dir_path = file_path;
                }
              }
              file_chooser.close();
            }
          });
        }
      });
      game_dir_box.append(&game_dir_choose_btn);
    }
    vbox.append(&game_dir_box);

    let mods_view = gtk::Box::new(gtk4::Orientation::Vertical, 10);
    {
      let localized_korabli = gtk::Box::new(gtk4::Orientation::Horizontal, 10);
      {
        let check_button = gtk::CheckButton::new();
        check_button.connect_toggled({
          let mods_to_install = RefCell::clone(&mods_to_install);
          move |button| {
            let mut mod_to_install = mods_to_install.borrow_mut();
            if button.is_active() {
              mod_to_install.push("localizedkorabli://game".to_string());
            } else if let Some(x) = mod_to_install
              .iter()
              .position(|x| x == "localizedkorabli://game")
            {
              mod_to_install.remove(x);
            }
          }
        });
        localized_korabli.append(&check_button);

        let name = gtk::Text::new();
        name.set_text("澪刻战舰世界莱服汉化");
        localized_korabli.append(&name);
      }
      mods_view.append(&localized_korabli);
    }
    vbox.append(&mods_view);

    let text_view = gtk::TextView::new();
    text_view.set_editable(false);
    text_view.set_vexpand(true);
    vbox.append(&text_view);

    let button = gtk::Button::with_label("安装/更新");
    button.connect_clicked({
      let window = window.to_owned();
      let text_view = text_view.to_owned();
      let mods_to_install = RefCell::clone(&mods_to_install);
      let event_queue = RefCell::clone(&event_queue);
      let game_dir_path = RefCell::clone(&game_dir_path);
      let seamonkey_cli_path = RefCell::clone(&seamonkey_cli_path);

      move |_| {
        let mods_to_install = mods_to_install.borrow().to_owned();
        match Command::new(seamonkey_cli_path.borrow().as_path())
          .arg("-yg")
          .arg(game_dir_path.borrow().as_path())
          .arg("install")
          .args(mods_to_install)
          .stdout(Stdio::piped())
          .spawn()
        {
          Ok(mut child) => {
            let buffer = text_view.buffer();
            buffer.set_text("");

            if let Some(stdout) = child.stdout.take() {
              std::thread::spawn({
                let event_queue = event_queue.to_owned();
                move || {
                  let mut reader = BufReader::new(stdout);
                  let mut all_buf = String::new();
                  let mut buf = String::new();
                  while let Ok(x) = reader.read_line(&mut buf) {
                    if x == 0 {
                      break;
                    }
                    all_buf += &buf;
                    event_queue
                      .borrow_mut()
                      .push_back(Event::LogUpdate(all_buf.to_owned()));
                  }
                  all_buf += "Mod更新已完成";
                  event_queue
                    .borrow_mut()
                    .push_back(Event::LogUpdate(all_buf.to_owned()));
                }
              });
            }
          }
          Err(err) => {
            let dialog = gtk::MessageDialog::builder()
              .transient_for(&window)
              .modal(true)
              .message_type(gtk::MessageType::Warning)
              .buttons(gtk::ButtonsType::Ok)
              .title("错误")
              .text("运行Mod管理器核心失败！")
              .build();

            eprintln!("{:?}", err);

            dialog.run_async(|dialog, _| {
              dialog.close();
            });
          }
        }
      }
    });
    vbox.append(&button);

    glib::source::timeout_add_local(Duration::from_millis(100), {
      let event_queue = RefCell::clone(&event_queue);
      let text_view = text_view.to_owned();
      move || {
        if let Some(ev) = event_queue.borrow_mut().pop_front() {
          match ev {
            Event::LogUpdate(log) => {
              text_view.buffer().set_text(&log);
            }
          }
        }
        glib::ControlFlow::Continue
      }
    });

    window.present();
  });

  application.run()
}
