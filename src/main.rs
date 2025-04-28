use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::glib::clone;
use async_channel::{Receiver, Sender};
use cloth::Cloth;
use drafting::{Draft, Quadstate};
use raylib::prelude::*;
mod cloth;
mod drafting;
use gtk::{
    Application, ApplicationWindow, CheckButton, FileChooserDialog, FileFilter, Label, ProgressBar,
    Separator, SpinButton, glib,
};
use gtk::{Box, Button, Notebook, prelude::*};

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
    Pin(bool),
    PinState(Quadstate),
    Render,
    Link(Option<u32>),
    RenderProgress(f64),
}

fn main() {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    app.run();
}

fn build_ui(app: &Application) {
    let (sender_for_raylib, receiver_for_gtk) = async_channel::bounded(1);
    let (sender_for_gtk, receiver_for_raylib): (Sender<Message>, Receiver<Message>) =
        async_channel::bounded(1);
    let sender_for_gtk = Rc::new(RefCell::new(sender_for_gtk));
    let sent_close = Arc::new(Mutex::new(false));

    let sent_close_copy = Arc::clone(&sent_close);
    app.connect_shutdown(clone!(
        #[strong]
        sender_for_gtk,
        move |_| {
            let mut sent_close_copy = sent_close_copy.lock().unwrap();
            if *sent_close_copy {
                return;
            }

            sender_for_gtk
                .borrow_mut()
                .send_blocking(Message::Close)
                .expect("The channel needs to be open.");
            *sent_close_copy = true;
        }
    ));

    gio::spawn_blocking(clone!(
        #[strong]
        sent_close,
        move || {
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
                first_down: Vector2::zero(),
                width: 10,
                height: 10,
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
                        Message::Pin(state) => {
                            draft.pin(state);
                        }

                        Message::OpenFile(file) => {
                            draft = Draft::new(file, WIDTH, HEIGHT);
                            state = State::Drafting;
                        }
                        Message::PinState(_) => {}
                        Message::Render => {
                            state = State::Rendering;
                            cloth =
                                Cloth::generate_from_draft(&draft, 0.1, 2.0, &sender_for_raylib);
                            paused = true;
                            rl.disable_cursor();
                        }
                        Message::RenderProgress(_) => {}
                        Message::Link(l) => {
                            draft.link(l);
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
                        if d.is_mouse_button_down(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT) {
                            sender_for_raylib
                                .send_blocking(Message::PinState(draft.get_pin_status()))
                                .expect("The channel needs to be open.");
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
            let mut sent_close_copy = sent_close.lock().unwrap();
            if *sent_close_copy {
                return;
            }
            sender_for_raylib
                .send_blocking(Message::Close)
                .expect("The channel needs to be open.");
            *sent_close_copy = true;
        }
    ));

    let settings = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    let design = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .width_request(300)
        .height_request(200)
        .margin_top(8)
        .build();

    let edit_container = Box::builder()
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .orientation(gtk::Orientation::Vertical)
        .visible(false)
        .build();

    let render_container = Box::builder()
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .orientation(gtk::Orientation::Vertical)
        .visible(false)
        .build();

    let progress_bar = ProgressBar::builder().width_request(200).build();
    progress_bar.set_show_text(true); // this isnt true by default TODO: make bug request
    progress_bar.set_text(Some("Rasterizing..."));

    let done_text = Label::builder().visible(false).build();
    done_text.set_text("Done!");
    let close_button = Button::builder().visible(false).build();
    close_button.set_label("Quit.");
    close_button.connect_clicked(clone!(
        #[strong]
        sender_for_gtk,
        move |_| {
            sender_for_gtk
                .borrow_mut()
                .send_blocking(Message::Close)
                .expect("The channel needs to be open.");
        }
    ));

    render_container.append(&progress_bar);
    render_container.append(&done_text);
    render_container.append(&close_button);

    let apply_button = Button::builder().build();
    apply_button.set_label("Back");

    apply_button.connect_clicked(clone!(
        #[strong]
        sender_for_gtk,
        move |_| {
            sender_for_gtk
                .borrow_mut()
                .send_blocking(Message::Pin(false))
                .expect("The channel needs to be open.");
        }
    ));

    let continue_button = Button::builder().build();
    continue_button.set_label("Render!");

    let link_label = Label::builder().margin_top(6).margin_bottom(6).build();
    link_label.set_label("Link Number");

    let current_spin_state = Rc::new(RefCell::new(0 as u32));
    let spin_button = SpinButton::builder().margin_top(6).margin_bottom(6).build();
    spin_button.set_range(0.0, 100.0);
    spin_button.set_increments(1.0, 100.0);
    spin_button.connect_value_changed(clone!(
        #[strong]
        current_spin_state,
        move |spin| {
            *current_spin_state.borrow_mut() = spin.value() as u32;
        }
    ));

    let pin_button = CheckButton::builder()
        .margin_top(6)
        .margin_bottom(6)
        .build();
    pin_button.set_label(Some("Pinned"));

    let link_button = Button::builder().build();
    link_button.set_label("Link");

    let current_pin_state = Rc::new(RefCell::new(Quadstate::No));
    pin_button.connect_toggled(clone!(
        #[strong]
        current_pin_state,
        move |pin| {
            *current_pin_state.borrow_mut() = if pin.is_active() {
                Quadstate::On
            } else {
                Quadstate::Off
            };
            pin.set_inconsistent(false);
        }
    ));

    let apply_button = Button::builder().build();
    apply_button.set_label("Apply");

    apply_button.connect_clicked(clone!(
        #[strong]
        sender_for_gtk,
        #[strong]
        current_pin_state,
        move |_| {
            let state = current_pin_state.borrow_mut();
            let set_val = match *state {
                Quadstate::On => true,
                _ => false,
            };
            sender_for_gtk
                .borrow_mut()
                .send_blocking(Message::Pin(set_val))
                .expect("The channel needs to be open.");
        }
    ));

    link_button.connect_clicked(clone!(
        #[strong]
        sender_for_gtk,
        #[strong]
        current_spin_state,
        move |_| {
            let state = current_spin_state.borrow_mut();
            sender_for_gtk
                .borrow_mut()
                .send_blocking(Message::Link(if *state != 0 { Some(*state) } else { None }))
                .expect("The channel needs to be open.");
        }
    ));

    edit_container.append(&pin_button);
    edit_container.append(&apply_button);

    edit_container.append(&link_label);
    edit_container.append(&spin_button);
    edit_container.append(&link_button);

    let seperator = Separator::builder()
        .margin_top(10)
        .margin_bottom(10)
        .build();
    edit_container.append(&seperator);
    edit_container.append(&continue_button);

    let upload_container = Box::builder()
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .build();
    let upload_button = Button::builder().build();
    upload_button.set_label("Load Design");

    let filter = FileFilter::new();
    filter.add_suffix("svg");

    let upload_dialog = FileChooserDialog::builder()
        .action(gtk::FileChooserAction::Open)
        .title("Pick a design")
        .filter(&filter)
        .build();
    upload_dialog.add_button("Open", gtk::ResponseType::Accept);
    upload_dialog.set_default_response(gtk::ResponseType::Accept);

    upload_container.append(&upload_button);

    upload_dialog.connect_response(clone!(
        #[strong]
        sender_for_gtk,
        move |dialog, response_type| {
            match response_type {
                gtk::ResponseType::Accept => match dialog.file() {
                    None => {}
                    Some(file_path) => {
                        sender_for_gtk
                            .borrow_mut()
                            .send_blocking(Message::OpenFile(
                                // this is bad. i dont gaf
                                file_path.path().unwrap().to_str().unwrap().to_string(),
                            ))
                            .expect("The channel needs to be open.");
                        dialog.destroy();
                    }
                },
                _ => {}
            }
        }
    ));

    design.append(&upload_container);
    design.append(&edit_container);
    design.append(&render_container);

    continue_button.connect_clicked(clone!(
        #[strong]
        sender_for_gtk,
        move |button| {
            print!("showing\n");
            render_container.show();
            button.parent().unwrap().hide();

            sender_for_gtk
                .borrow_mut()
                .send_blocking(Message::Render)
                .expect("The channel needs to be open.");
        }
    ));

    upload_button.connect_clicked(move |_| {
        upload_dialog.present();
        upload_container.hide();
        edit_container.show();
    });

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

    pin_button.hide();
    link_label.hide();
    spin_button.hide();
    link_button.hide();
    glib::spawn_future_local(clone!(
        #[weak]
        app,
        async move {
            while let Ok(message) = receiver_for_gtk.recv().await {
                match message {
                    Message::Close => {
                        app.active_window().unwrap().close();
                    }
                    Message::OpenFile(file) => {
                        todo!()
                    }
                    Message::Pin(_) => {
                        todo!()
                    }
                    Message::PinState(state) => {
                        pin_button.show();
                        link_label.show();
                        spin_button.show();
                        link_button.show();
                        pin_button.set_inconsistent(false);

                        match state {
                            Quadstate::On => {
                                pin_button.set_active(true);
                            }
                            Quadstate::Maybe => {
                                pin_button.set_inconsistent(true);
                            }
                            Quadstate::Off => {
                                pin_button.set_active(false);
                            }
                            Quadstate::No => {
                                pin_button.hide();
                                link_label.hide();
                                spin_button.hide();
                                link_button.hide();
                            }
                        }

                        *current_pin_state.borrow_mut() = state;
                    }
                    Message::RenderProgress(prog) => {
                        print!("prog: {}%\n", prog * 100.0);
                        progress_bar.set_fraction(prog as f64);
                        if prog == 1.0 {
                            done_text.show();
                            close_button.show();
                            progress_bar.hide();
                        }
                    }
                    Message::Render => {}
                    Message::Link(_) => {}
                }
            }
        }
    ));
    window.present();
}
