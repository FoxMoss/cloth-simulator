use std::cell::Cell;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

use crate::glib::clone;
use async_channel::{Receiver, Sender};
use cloth::Cloth;
use drafting::Draft;
use gio::glib::subclass::object::ObjectImpl;
use gio::glib::{Properties, Value, property};
use gio::subclass::prelude::ApplicationImpl;
use raylib::prelude::*;
mod cloth;
mod drafting;
use gtk::{Application, ApplicationWindow, FileChooserDialog, FileFilter, Label, glib};
use gtk::{Box, Button, Notebook, prelude::*};
use std::cell;

const APP_ID: &str = "org.foxmoss.Weaverling";

const WIDTH: i32 = 640;
const HEIGHT: i32 = 480;
enum State {
    FilePicker,
    Drafting,
    Rendering,
}

enum Message {
    Close,
    OpenFile(String),
}

fn main() {
    let app = Application::builder().application_id(APP_ID).build();

    let (sender, receiver) = async_channel::bounded(1);
    let (sender_for_gtk, receiver_for_raylib): (Sender<Message>, Receiver<Message>) =
        async_channel::bounded(1);
    let sender_for_gtk = Arc::new(Mutex::new(sender_for_gtk));

    unsafe {
        app.set_data("sender_for_gtk", Arc::clone(&sender_for_gtk));
        app.set_data("closed_sent", false);
    }

    app.connect_activate(build_ui);

    gio::spawn_blocking(move || {
        let (mut rl, thread) = raylib::init()
            .size(WIDTH, HEIGHT)
            .title("Weaverling")
            .build();

        let mut draft = Draft {
            lines: vec![],
            camera: Camera2D {
                offset: Vector2 {
                    x: (WIDTH / 2) as f32,
                    y: (HEIGHT / 2) as f32,
                },
                target: Vector2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
                zoom: 5.0,
            },
            current_link: 1,
        };

        let mut state = State::FilePicker;

        let mut cam = camera::Camera3D::perspective(
            Vector3 {
                x: 0.0,
                y: 10.0,
                z: 10.0,
            },
            Vector3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Vector3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            45.0,
        );

        let mut cloth = cloth::Cloth::generate_square(10, 10, 0.5);

        rl.set_target_fps(30);

        let mut paused = true;

        while !rl.window_should_close() {
            let message = receiver_for_raylib.try_recv();
            match message {
                Ok(message_body) => match message_body {
                    Message::Close => {
                        break;
                    }
                    Message::OpenFile(file) => {
                        draft = Draft::new(file, WIDTH, HEIGHT);
                        state = State::Drafting;
                    }
                },
                _ => {}
            }
            let mut d = rl.begin_drawing(&thread);
            d.clear_background(Color::WHITE);

            match state {
                State::FilePicker => {
                    d.draw_text("Waiting for file...", 20, 20, 30, Color::BLACK);
                }
                State::Drafting => {
                    if d.is_key_pressed(KeyboardKey::KEY_ENTER) {
                        state = State::Rendering;
                        cloth = Cloth::generate_from_draft(&draft, 0.1, 1.4);
                        paused = true;
                        d.disable_cursor();
                    }
                    draft.draw(&mut d);
                }
                State::Rendering => {
                    d.update_camera(&mut cam, CameraMode::CAMERA_FREE);

                    if paused {
                        d.draw_text("paused", 10, 400, 1, raylib::color::Color::BLACK);
                    }
                    if d.is_key_pressed(KeyboardKey::KEY_P) {
                        paused = !paused;
                    }

                    {
                        let mut r = d.begin_mode3D(cam);
                        cloth.draw(&mut r);
                    }
                    d.draw_fps(0, 0);

                    if !paused {
                        cloth.step();
                    }

                    if d.is_key_pressed(KeyboardKey::KEY_ENTER) {
                        state = State::Drafting;
                        d.enable_cursor();
                    }
                }
            }
        }
        sender
            .send_blocking(Message::Close)
            .expect("The channel needs to be open.");
    });

    glib::spawn_future_local(clone!(
        #[weak]
        app,
        async move {
            while let Ok(message) = receiver.recv().await {
                match message {
                    Message::Close => {
                        unsafe {
                            app.set_data("closed_sent", true);
                        }
                        app.active_window().unwrap().close();
                    }
                    Message::OpenFile(file) => {
                        todo!()
                    }
                }
            }
        }
    ));

    app.run();
}

fn build_ui(app: &Application) {
    let sender = unsafe { app.data::<Arc<Mutex<Sender<Message>>>>("sender_for_gtk") };

    if let Some(mut sender) = sender {
        unsafe {
            let sender_clone = Arc::clone(sender.as_mut());

            app.connect_shutdown(move |_| {
                let sender = sender_clone.lock().unwrap();
                sender
                    .send_blocking(Message::Close)
                    .expect("The channel needs to be open.");
            });
        }
    } else {
        eprintln!("Failed to retrieve sender from application data.");
    }

    let settings = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    let design = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .width_request(600)
        .height_request(600)
        .build();

    let upload_container = Box::builder()
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .build();
    let upload_button = Button::builder().build();
    upload_button.set_label("Upload Design");

    let filter = FileFilter::new();
    filter.add_suffix("svg");

    let upload_dialog_button = Button::builder().build();
    let upload_dialog = FileChooserDialog::builder()
        .action(gtk::FileChooserAction::Open)
        .title("Pick a design")
        .filter(&filter)
        .build();
    upload_dialog.add_button("Open", gtk::ResponseType::Accept);
    upload_dialog.set_default_response(gtk::ResponseType::Accept);

    if let Some(mut sender) = sender {
        unsafe {
            let sender_clone = Arc::clone(sender.as_mut());

            upload_dialog.connect_response(move |dialog, response_type| {
                match response_type {
                    gtk::ResponseType::Accept => match dialog.file() {
                        None => {}
                        Some(file_path) => {
                            let sender = sender_clone.lock().unwrap();
                            sender
                                .send_blocking(Message::OpenFile(
                                    file_path.path().unwrap().to_str().unwrap().to_string(),
                                    // this is bad. i dont gaf
                                ))
                                .expect("The channel needs to be open.");
                            dialog.destroy();
                        }
                    },
                    _ => {}
                }
            });
        }
    } else {
        eprintln!("Failed to retrieve sender from application data.");
    }

    upload_button.connect_clicked(move |_| {
        upload_dialog.present();
    });
    upload_container.append(&upload_button);
    design.append(&upload_container);

    let notebook = Notebook::builder().build();
    let design_tab = Label::builder().build();
    design_tab.set_label("Design");
    notebook.append_page(&design, Some(&design_tab));

    let settings_tab = Label::builder().build();
    settings_tab.set_label("Settings");
    notebook.append_page(&settings, Some(&settings_tab));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Weaverling")
        .child(&notebook)
        .build();

    window.present();
}
