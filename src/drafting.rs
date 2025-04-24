use core::f32;
use raylib::{
    camera::Camera2D,
    color::Color,
    math::Vector2,
    prelude::{RaylibDraw, RaylibDrawHandle, RaylibMode2DExt},
};
use std::fs::File;
use std::io::BufReader;
use xml::{
    common::Position,
    reader::{EventReader, XmlEvent},
};

#[derive(PartialEq, Clone, Copy)]
pub struct Line {
    pub p1: Vector2,
    pub p2: Vector2,
    pub pinned: bool,
    pub link: Option<u32>,
    pub highlighted: bool,
}

pub enum Quadstate {
    On,
    Maybe,
    Off,
    No,
}

impl Line {
    // https://en.wikipedia.org/wiki/Distance_from_a_point_to_a_line#Line_defined_by_two_points
    pub fn dist(&self, point: Vector2) -> f32 {
        ((self.p2.y - self.p1.y) * point.x - (self.p2.x - self.p1.x) * point.y
            + self.p2.x * self.p1.y
            - self.p2.y * self.p1.x)
            .abs()
            / ((self.p2.y - self.p1.y).powi(2) + (self.p2.x - self.p1.x).powi(2)).sqrt()
    }
    pub fn in_slice(&self, point: Vector2, threshold: f32) -> bool {
        if self.p1.x - self.p2.x > 0.0 {
            return point.x < self.p1.x + threshold && point.x > self.p2.x - threshold;
        } else {
            return point.x < self.p2.x + threshold && point.x > self.p1.x - threshold;
        }
    }
    pub fn get_intersect_on_x(&self, point: Vector2) -> Option<f32> {
        if (self.p2.x - self.p1.x) == 0.0 {
            return None; // divsion by 0
        }
        Some(self.p1.y + (point.x - self.p1.x) * (self.p2.y - self.p1.y) / (self.p2.x - self.p1.x))
    }
    //https://stackoverflow.com/a/2752754
    pub fn hitbox(&self, point: Vector2, threshold: f32) -> bool {
        let ray = self.p2 - self.p1;
        let normal = Vector2 {
            x: -ray.y,
            y: ray.x,
        }
        .normalized()
            * threshold;
        return {
            let p2 = self.p2 + normal;
            let p1 = self.p1 + normal;

            ((p2.x - p1.x) * (point.y - p1.y) - (point.x - p1.x) * (p2.y - p1.y)) < 0.0
        } && {
            let p2 = self.p2 - normal;
            let p1 = self.p1 - normal;

            ((p2.x - p1.x) * (point.y - p1.y) - (point.x - p1.x) * (p2.y - p1.y)) > 0.0
        } && {
            let p2 = self.p2 + normal;
            let p1 = self.p2 - normal;

            ((p2.x - p1.x) * (point.y - p1.y) - (point.x - p1.x) * (p2.y - p1.y)) > 0.0
        } && {
            let p2 = self.p1 + normal;
            let p1 = self.p1 - normal;

            ((p2.x - p1.x) * (point.y - p1.y) - (point.x - p1.x) * (p2.y - p1.y)) < 0.0
        };
    }
    // same shape
    pub fn partial_match(&self, other: &Line) -> bool {
        self.p1 == other.p1 || self.p1 == other.p2 || self.p2 == other.p1 || self.p2 == other.p2
    }
}

fn rect_colision(root: Vector2, size: Vector2, point: Vector2) -> bool {
    let biggest_y = root.y.max(root.y + size.y);
    let smallest_y = root.y.min(root.y + size.y);
    let biggest_x = root.x.max(root.x + size.x);
    let smallest_x = root.x.min(root.x + size.x);
    return point.x < biggest_x
        && point.x > smallest_x
        && point.y < biggest_y
        && point.y > smallest_y;
}

pub struct Draft {
    pub lines: Vec<Line>,
    pub camera: Camera2D,
    pub current_link: u32,
    pub first_down: Vector2,
    pub width: i32,
    pub height: i32,
}

impl Draft {
    pub fn new(file: String, width: i32, height: i32) -> Draft {
        let mut draft = Draft {
            lines: vec![],
            camera: Camera2D {
                offset: Vector2 {
                    x: (width / 2) as f32,
                    y: (height / 2) as f32,
                },
                target: Vector2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
                zoom: 5.0,
            },
            current_link: 1,
            first_down: Vector2::zero(),
            width,
            height,
        };

        let file = File::open(file).unwrap();
        let file = BufReader::new(file);
        let parser = EventReader::new(file);
        let mut depth = 0;
        for e in parser {
            match e {
                Ok(XmlEvent::StartElement {
                    name, attributes, ..
                }) => {
                    let local_name = name.local_name;

                    let mut p1 = Vector2 { x: 0.0, y: 0.0 };
                    let mut p2 = Vector2 { x: 0.0, y: 0.0 };
                    let is_line = local_name == "line";

                    for attr in attributes {
                        let attr_name = attr.name;
                        let attr_value = attr.value;
                        match attr_name.local_name.as_str() {
                            "x1" => p1.x = attr_value.parse::<f32>().unwrap(),
                            "y1" => p1.y = attr_value.parse::<f32>().unwrap(),
                            "x2" => p2.x = attr_value.parse::<f32>().unwrap(),
                            "y2" => p2.y = attr_value.parse::<f32>().unwrap(),
                            _ => {}
                        }
                    }

                    if is_line {
                        draft.lines.push(Line {
                            p1,
                            p2,
                            pinned: false,
                            link: None,
                            highlighted: false,
                        });
                    }
                    depth += 1;
                }
                Ok(XmlEvent::EndElement { name }) => {
                    depth -= 1;
                }
                _ => {}
            }
        }
        return draft;
    }

    pub fn get_bounds(&self) -> (Vector2, Vector2) {
        let mut min_num = Vector2 {
            x: f32::INFINITY,
            y: f32::INFINITY,
        };
        let mut max_num = Vector2 {
            x: -f32::INFINITY,
            y: -f32::INFINITY,
        };
        for line in &self.lines {
            min_num.x = min_num.x.min(line.p1.x.min(line.p2.x));
            min_num.y = min_num.y.min(line.p1.y.min(line.p2.y));
            max_num.x = max_num.x.max(line.p1.x.max(line.p2.x));
            max_num.y = max_num.y.max(line.p1.y.max(line.p2.y));
        }
        return (
            min_num - Vector2 { x: 1.0, y: 1.0 },
            max_num + Vector2 { x: 1.0, y: 1.0 },
        );
    }

    pub fn draw(&mut self, d: &mut RaylibDrawHandle) {
        let move_camera = (d.is_key_down(raylib::ffi::KeyboardKey::KEY_LEFT_CONTROL)
            || d.is_key_down(raylib::ffi::KeyboardKey::KEY_SPACE));
        if d.is_mouse_button_down(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT) && move_camera {
            let mut delta = d.get_mouse_delta();
            delta.scale(-1.0 / self.camera.zoom);
            self.camera.target += delta;
        }
        let wheel = d.get_mouse_wheel_move();
        let mouse_world_pos = d.get_screen_to_world2D(d.get_mouse_position(), self.camera);
        self.camera.offset = d.get_mouse_position();
        self.camera.target = mouse_world_pos;
        let scale = 0.2 * wheel;
        self.camera.zoom = self.camera.zoom + scale;

        d.draw_text(
            format!("Link number: {}", self.current_link).as_str(),
            10,
            40,
            20,
            Color::BLACK,
        );

        let mut m = d.begin_mode2D(self.camera);

        if m.is_mouse_button_pressed(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT) && !move_camera {
            if !m.is_key_down(raylib::ffi::KeyboardKey::KEY_LEFT_SHIFT) {
                for line in &mut self.lines {
                    line.highlighted = false;
                }
            }
            self.first_down = mouse_world_pos;
        }
        if m.is_mouse_button_down(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT) && !move_camera {
            let biggest_y = mouse_world_pos.y.max(self.first_down.y);
            let smallest_y = mouse_world_pos.y.min(self.first_down.y);
            let biggest_x = mouse_world_pos.x.max(self.first_down.x);
            let smallest_x = mouse_world_pos.x.min(self.first_down.x);

            m.draw_rectangle_v(
                Vector2 {
                    x: smallest_x,
                    y: smallest_y,
                },
                Vector2 {
                    x: biggest_x - smallest_x,
                    y: biggest_y - smallest_y,
                },
                Color::GRAY,
            );
        }

        for line in &mut self.lines {
            if m.is_mouse_button_down(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT)
                && !move_camera
                && (rect_colision(self.first_down, mouse_world_pos - self.first_down, line.p1)
                    || rect_colision(self.first_down, mouse_world_pos - self.first_down, line.p2))
            {
                line.highlighted = true;
            } else if m.is_mouse_button_down(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT)
                && !move_camera
                && !m.is_key_down(raylib::ffi::KeyboardKey::KEY_LEFT_SHIFT)
                && !line.hitbox(mouse_world_pos, 7.0 * (1.0 / self.camera.zoom))
            {
                line.highlighted = false;
            }
            m.draw_line_v(
                line.p1,
                line.p2,
                if line.hitbox(mouse_world_pos, 7.0 * (1.0 / self.camera.zoom)) {
                    Color::DARKRED
                } else if line.highlighted {
                    Color::RED
                } else if line.pinned {
                    Color::ORANGE
                } else {
                    Color::GREEN
                },
            );
            if m.is_mouse_button_pressed(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT)
                && line.hitbox(mouse_world_pos, 7.0 * (1.0 / self.camera.zoom))
            {
                line.highlighted = true;
            }
            match line.link {
                None => {}
                Some(link) => {
                    m.draw_text(
                        format!("{}", link).as_str(),
                        (line.p1.x + (line.p2.x - line.p1.x) / 2.0) as i32,
                        (line.p1.y + (line.p2.y - line.p1.y) / 2.0) as i32,
                        1,
                        Color::BLACK,
                    );
                }
            }
        }
    }
    pub fn pin(&mut self, to: bool) {
        for line in &mut self.lines {
            if line.highlighted {
                line.pinned = to;
            }
        }
    }
    pub fn get_pin_status(&mut self) -> Quadstate {
        let mut all_false = true;
        let mut all_true = true;
        let mut selected_count = 0;

        for line in &mut self.lines {
            if line.highlighted && line.pinned {
                all_false = false;
            } else if line.highlighted && !line.pinned {
                all_true = false;
            }
            if line.highlighted {
                selected_count += 1;
            }
        }
        if selected_count == 0 {
            Quadstate::No
        } else if all_true {
            Quadstate::On
        } else if all_false {
            Quadstate::Off
        } else {
            Quadstate::Maybe
        }
    }
}
