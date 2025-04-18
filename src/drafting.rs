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

pub struct Draft {
    pub lines: Vec<Line>,
    camera: Camera2D,
    current_link: u32,
}

impl Draft {
    pub fn new(file: &str, width: i32, height: i32) -> Draft {
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
        if d.is_mouse_button_down(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT) {
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

        let mut pin = false;
        if d.is_key_pressed(raylib::ffi::KeyboardKey::KEY_S) {
            pin = true;
        }
        let mut link = false;
        if d.is_key_pressed(raylib::ffi::KeyboardKey::KEY_F) {
            link = true;
        }
        if d.is_key_pressed(raylib::ffi::KeyboardKey::KEY_Q) {
            self.current_link += 1;
        }
        if d.is_key_pressed(raylib::ffi::KeyboardKey::KEY_E) {
            self.current_link -= 1;
        }

        d.draw_text(
            format!("Link number: {}", self.current_link).as_str(),
            10,
            40,
            20,
            Color::BLACK,
        );

        let mut m = d.begin_mode2D(self.camera);
        for line in &mut self.lines {
            m.draw_line_v(
                line.p1,
                line.p2,
                if line.pinned {
                    Color::RED
                } else {
                    Color::GREEN
                },
            );
            if line.hitbox(mouse_world_pos, 2.0 * self.camera.zoom) {
                if pin {
                    line.pinned = !line.pinned;
                }
                if link {
                    line.link = match line.link {
                        None => Some(self.current_link),
                        Some(_) => None,
                    }
                }
                m.draw_line_v(line.p1, line.p2, Color::BLUE);
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
}
