use cloth::Cloth;
use drafting::Draft;
use raylib::prelude::*;
mod cloth;
mod drafting;

enum State {
    Drafting,
    Rendering,
}

const WIDTH: i32 = 640;
const HEIGHT: i32 = 480;

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title("Weaverling")
        .build();

    let mut draft = Draft::new("files/BagProto.svg", WIDTH, HEIGHT);

    let mut state = State::Drafting;

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

    let mut cloth = cloth::Cloth::generate_square(10, 10, 0.1);

    rl.set_target_fps(30);

    let mut paused = true;

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::WHITE);

        match state {
            State::Drafting => {
                if d.is_key_pressed(KeyboardKey::KEY_ENTER) {
                    state = State::Rendering;
                    cloth = Cloth::generate_from_draft(&draft, 0.1, 1.4);
                    d.disable_cursor();
                    paused = true;
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
}
