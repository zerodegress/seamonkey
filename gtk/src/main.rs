use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow};
use gtk4 as gtk;

fn main() -> glib::ExitCode {
  env_logger::init();

  let application = Application::builder()
    .application_id("ink.zerodgress.seamonkey.gtk")
    .build();

  application.connect_activate(|app| {
    let mods_to_install = Arc::new(Mutex::new(Vec::<String>::new()));

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

    let mods_view = gtk::Box::new(gtk4::Orientation::Vertical, 10);
    {
      let localized_korabli = gtk::Box::new(gtk4::Orientation::Horizontal, 10);
      {
        let check_button = gtk::CheckButton::new();
        check_button.connect_toggled({
          let mods_to_install = Arc::clone(&mods_to_install);
          move |button| {
            let mut mod_to_install = mods_to_install.lock().expect("wtf lock failed");
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
    text_view.set_vexpand(true);
    vbox.append(&text_view);

    let button = gtk::Button::with_label("安装/更新");
    button.connect_clicked({
      let window = window.to_owned();
      let text_view = text_view.to_owned();
      let mods_to_install = Arc::clone(&mods_to_install);
      move |_| {
        let mods_to_install = mods_to_install.lock().expect("wtf lock failed").to_owned();
        match Command::new("./target/debug/seamonkey_cli")
          .arg("-yg")
          .arg(".temp/fake-game")
          .arg("install")
          .args(mods_to_install)
          .stdout(Stdio::piped())
          .spawn()
        {
          Ok(mut child) => {
            let buffer = text_view.buffer();
            buffer.set_text("");

            if let Some(stdout) = child.stdout.take() {
              println!("az");
              let mut reader = BufReader::new(stdout);
              let mut buf = String::new();
              while let Ok(x) = reader.read_line(&mut buf) {
                if x == 0 {
                  break;
                }
                buffer.set_text(&buf);
              }
            }

            if let Err(err) = child.wait() {
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

            let dialog = gtk::MessageDialog::builder()
              .transient_for(&window)
              .modal(true)
              .message_type(gtk::MessageType::Info)
              .buttons(gtk::ButtonsType::Ok)
              .title("完成！")
              .text("Mod已安装/更新")
              .build();

            dialog.run_async(|dialog, _| {
              dialog.close();
            });
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

    window.present();
  });

  application.run()
}
