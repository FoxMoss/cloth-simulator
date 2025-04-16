use std::{clone, collections::HashMap};

use raylib::prelude::*;

use crate::drafting::{Draft, Line};

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Index3 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Index3 {
    pub fn get_neighbors(&self) -> Vec<Self> {
        let neighbors: Vec<Self> = vec![
            Self {
                x: self.x + 1,
                y: self.y,
                z: self.z,
            },
            Self {
                x: self.x - 1,
                y: self.y,
                z: self.z,
            },
            Self {
                x: self.x,
                y: self.y + 1,
                z: self.z,
            },
            Self {
                x: self.x,
                y: self.y - 1,
                z: self.z,
            },
            Self {
                x: self.x,
                y: self.y,
                z: self.z + 1,
            },
            Self {
                x: self.x,
                y: self.y,
                z: self.z - 1,
            },
        ];
        neighbors
    }
}

// A theoretical square patch of fabric
#[derive(Clone, Copy)]
pub struct ClothSegmentFrag {
    index: Index3,
    position: Vector3,
    pub velocity: Vector3,
    pinned: bool,
    pub link_vector: Option<Vector2>,
}
pub struct ClothSegment {
    frag: ClothSegmentFrag,
    neighbors: Vec<Index3>,
    neighbor_index: Vec<usize>,
    pub link_number: Option<u32>,
}

pub struct Cloth {
    pub segments: Vec<ClothSegment>,
    scale: f32,
}

impl Cloth {
    pub fn generate_square(width: i32, height: i32, scale: f32) -> Self {
        let mut segments: Vec<ClothSegment> = vec![];
        for x in 0..width {
            for y in 0..height {
                segments.push(ClothSegment {
                    frag: ClothSegmentFrag {
                        index: Index3 { x, y: 0, z: y },
                        position: Vector3 {
                            x: x.as_f32() * scale,
                            y: 1.0,
                            z: y.as_f32() * scale,
                        },
                        velocity: Vector3::zero(),
                        pinned: x == 0,
                        link_vector: None,
                    },
                    neighbors: vec![],
                    neighbor_index: vec![],
                    link_number: None,
                });
            }
        }
        let mut ret = Cloth { segments, scale };
        for x in 0..width {
            for y in 0..height {
                let (segment, neighbors, neighbor_index) =
                    ret.find_index_info(Index3 { x, y: 0, z: y }).unwrap();
                segment.neighbors = neighbors;
                segment.neighbor_index = neighbor_index;
            }
        }
        ret
    }
    pub fn generate_from_draft(draft: &Draft, scale: f32, detail: f32) -> Self {
        let mut segments: Vec<ClothSegment> = vec![];
        let mut segment_links: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut segment_frags: Vec<ClothSegmentFrag> = vec![];

        let (min_bound, max_bound) = draft.get_bounds();
        // since rust doesnt have good for loops i have to do this ugly syntax
        let mut x = min_bound.x;
        let mut x_step = 0;
        let mut y_index_max = 0;
        let mut insert_index: u32 = 0;
        while x < max_bound.x {
            let mut check = Vector2 { x, y: max_bound.y };
            let mut intersections: u32 = 0;
            let mut confirmed_lines: Vec<Line> = vec![];
            let mut y_step = 0;
            while check.y > min_bound.y {
                let mut intersected = false;
                let mut pinned = false;
                let mut link_vector: Option<Vector2> = None;
                let mut link_number: Option<u32> = None;
                'draft_lines: for line in &draft.lines {
                    if line.hitbox(check, detail + 0.001) {
                        if line.pinned {
                            pinned = true;
                        }
                    }
                    if line.p1.x == line.p2.x {
                        continue;
                    }
                    // rust has no goto satement so you have to do this ugly
                    if line.hitbox(check, detail + 0.001) {
                        if line.pinned {
                            pinned = true;
                        }
                        if line.link.is_some() {
                            link_vector = Some(line.p1);
                            link_number = line.link;
                        }

                        for checked_line in &confirmed_lines {
                            if checked_line.partial_match(line) {
                                continue 'draft_lines;
                            }
                        }

                        confirmed_lines.push(*line);
                        intersected = true;
                    }
                }
                if intersections % 2 == 1 {
                    let frag = ClothSegmentFrag {
                        index: Index3 {
                            x: x_step,
                            y: 0,
                            z: y_step,
                        },
                        position: Vector3 {
                            x: x_step.as_f32() * scale,
                            y: 1.0,
                            z: y_step.as_f32() * scale,
                        },
                        velocity: Vector3::zero(),
                        pinned,
                        link_vector,
                    };
                    segments.push(ClothSegment {
                        frag,
                        neighbors: vec![],
                        neighbor_index: vec![],
                        link_number,
                    });
                    segment_frags.push(frag);
                    if link_number.is_some() {
                        let number = link_number.unwrap();
                        if !segment_links.contains_key(&number) {
                            segment_links.insert(number, vec![]);
                        }
                        segment_links.get_mut(&number).unwrap().push(insert_index);
                    }
                    insert_index += 1;
                }
                intersections += intersected as u32;

                check.y -= detail;
                y_step += 1;
            }
            y_index_max = y_step;

            x += detail;
            x_step += 1;
        }
        let x_index_max = x_step;

        let mut ret = Cloth { segments, scale };
        for x in 0..x_index_max {
            for y in 0..y_index_max {
                match ret.find_index_info(Index3 { x, y: 0, z: y }) {
                    None => {}
                    Some((segment, neighbors, neighbor_index)) => {
                        segment.neighbors = neighbors;
                        segment.neighbor_index = neighbor_index;

                        if segment.frag.link_vector.is_some() {
                            let number = segment.link_number.unwrap();
                            for index in segment_links[&number].clone() {
                                segment.neighbors.push(segment_frags[index as usize].index);
                                segment.neighbor_index.push(index.try_into().unwrap());
                            }
                        }
                    }
                }
            }
        }
        ret
    }

    pub fn draw(&self, r: &mut RaylibMode3D<'_, RaylibDrawHandle<'_>>) {
        for segment in &self.segments {
            r.draw_cube(
                segment.frag.position,
                self.scale,
                self.scale,
                self.scale,
                if segment.frag.pinned {
                    color::Color::RED
                } else {
                    color::Color::GREEN
                },
            );
        }
    }
    pub fn step(&mut self) {
        let mut segment_memory: Vec<ClothSegmentFrag> = vec![];
        for segment in &self.segments {
            segment_memory.push(segment.frag);
        }

        for segment in self.segments.iter_mut() {
            segment.frag.velocity += Vector3 {
                x: 0.0,
                y: -0.0006,
                z: 0.0,
            };

            segment.frag.velocity /= 1.01;
            for index in &segment.neighbor_index {
                let frag = segment_memory[*index];

                let diff = segment.frag.position - frag.position;
                let change = self.scale - diff.length();
                segment.frag.velocity += diff.normalized().scale_by(change * 0.3);
            }
            let new_max = segment.frag.velocity.length().min(0.5);
            segment.frag.velocity = segment.frag.velocity.normalized() * new_max;

            if !segment.frag.pinned {
                segment.frag.position += segment.frag.velocity;
            }
        }
    }
    fn get_neighbors(&self, index: Index3) -> (Vec<Index3>, Vec<usize>) {
        let mut ret: Vec<Index3> = vec![];
        let mut ret_index: Vec<usize> = vec![];
        let neighbors = index.get_neighbors();
        let mut neighbor_index = 0;
        for segment in &self.segments {
            if neighbors.contains(&segment.frag.index) {
                ret.push(segment.frag.index);
                ret_index.push(neighbor_index);
            }
            neighbor_index += 1;
        }
        (ret, ret_index)
    }
    pub fn find(&mut self, index: Index3) -> Option<&mut ClothSegment> {
        for segment in self.segments.iter_mut() {
            if segment.frag.index == index {
                return Some(segment);
            }
        }
        return None;
    }

    fn find_index_info(
        &mut self,
        index: Index3,
    ) -> Option<(&mut ClothSegment, Vec<Index3>, Vec<usize>)> {
        let neighbors = self.get_neighbors(index);
        for segment in self.segments.iter_mut() {
            if segment.frag.index == index {
                return Some((segment, neighbors.0, neighbors.1));
            }
        }
        return None;
    }
}
