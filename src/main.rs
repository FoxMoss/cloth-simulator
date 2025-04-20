use cloth::Cloth;
use drafting::Draft;
use gtk::glib::Properties;
use gtk::glib::subclass::object::ObjectImpl;
use gtk::glib::subclass::types::ObjectSubclass;
use gtk::subclass::box_::BoxImpl;
use gtk::subclass::widget::WidgetImpl;
use raylib::prelude::*;
mod cloth;
mod drafting;
use gtk::{Application, ApplicationWindow, FileChooserDialog, FileFilter, Label, glib};
use gtk::{Box, Button, CenterBox, GLArea, Notebook, NotebookTab, prelude::*};

const APP_ID: &str = "org.foxmoss.Weaverling";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    app.run()
}

enum State {
    FilePicker,
    Drafting(String),
    Rendering,
}
fn build_ui(app: &Application) {
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
    upload_dialog.connect_response(|dialog, response_type| {
        match response_type {
            gtk::ResponseType::Accept => match dialog.file() {
                None => {}
                Some(file_path) => {}
            },
            _ => {}
        }
        dialog.destroy();
    });

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

const WIDTH: i32 = 640;
const HEIGHT: i32 = 480;

// fn main() {
//     let (mut rl, thread) = raylib::init()
//         .size(WIDTH, HEIGHT)
//         .title("Weaverling")
//         .build();
//
//     let mut draft = Draft::new("files/BagProto.svg", WIDTH, HEIGHT);
//
//     let mut state = State::Drafting;
//
//     let mut cam = camera::Camera3D::perspective(
//         Vector3 {
//             x: 0.0,
//             y: 10.0,
//             z: 10.0,
//         },
//         Vector3 {
//             x: 0.0,
//             y: 0.0,
//             z: 0.0,
//         },
//         Vector3 {
//             x: 0.0,
//             y: 1.0,
//             z: 0.0,
//         },
//         45.0,
//     );
//
//     let mut cloth = cloth::Cloth::generate_square(10, 10, 0.5);
//
//     rl.set_target_fps(30);
//
//     let mut paused = true;
//
//     while !rl.window_should_close() {
//         let mut d = rl.begin_drawing(&thread);
//         d.clear_background(Color::WHITE);
//
//         match state {
//             State::Drafting => {
//                 if d.is_key_pressed(KeyboardKey::KEY_ENTER) {
//                     state = State::Rendering;
//                     cloth = Cloth::generate_from_draft(&draft, 0.1, 1.4);
//                     d.disable_cursor();
//                     paused = true;
//                 }
//                 draft.draw(&mut d);
//             }
//             State::Rendering => {
//                 d.update_camera(&mut cam, CameraMode::CAMERA_FREE);
//
//                 if paused {
//                     d.draw_text("paused", 10, 400, 1, raylib::color::Color::BLACK);
//                 }
//                 if d.is_key_pressed(KeyboardKey::KEY_P) {
//                     paused = !paused;
//                 }
//
//                 {
//                     let mut r = d.begin_mode3D(cam);
//                     cloth.draw(&mut r);
//                 }
//                 d.draw_fps(0, 0);
//
//                 if !paused {
//                     cloth.step();
//                 }
//
//                 if d.is_key_pressed(KeyboardKey::KEY_ENTER) {
//                     state = State::Drafting;
//                     d.enable_cursor();
//                 }
//             }
//         }
//     }
// }
