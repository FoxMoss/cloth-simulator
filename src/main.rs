#![feature(core_intrinsics)]
use raylib::{ffi::DrawText, prelude::*};
mod cloth;

fn main() {
    let (mut rl, thread) = raylib::init().size(640, 480).title("Hello, World").build();

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

    rl.disable_cursor();
    rl.set_target_fps(30);

    let mut paused = true;

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);

        d.update_camera(&mut cam, CameraMode::CAMERA_FREE);

        d.clear_background(Color::WHITE);

        if paused {
            d.draw_text("paused", 10, 400, 1, raylib::color::Color::BLACK);
        }
        if d.is_key_pressed(KeyboardKey::KEY_P) {
            paused = !paused;
        }

        {
            let mut r = d.begin_mode3D(cam);
            r.draw_grid(10, 1.0);
            cloth.draw(&mut r);
        }
        d.draw_fps(0, 0);

        if !paused {
            cloth.step();
        }
    }
}
