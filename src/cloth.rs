use raylib::prelude::*;
use std::ops::Sub;
use std::{clone, collections::HashMap, f32, usize};

use crate::drafting::{Draft, Line};

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Index3 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Sub for Index3 {
    type Output = Index3;
    fn sub(self, v: Index3) -> Self {
        Index3 {
            x: self.x - v.x,
            y: self.y - v.y,
            z: self.z - v.z,
        }
    }
}
impl Index3 {
    pub fn length(&self) -> f32 {
        ((self.x.pow(2) + self.y.pow(2) + self.z.pow(2)) as f32).sqrt()
    }
}

impl Index3 {
    pub fn get_neighbors(&self) -> (Vec<Self>, Vec<Self>) {
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
        let second_neighbors: Vec<Self> = vec![
            Self {
                x: self.x + 2,
                y: self.y,
                z: self.z,
            },
            Self {
                x: self.x - 2,
                y: self.y,
                z: self.z,
            },
            Self {
                x: self.x,
                y: self.y + 2,
                z: self.z,
            },
            Self {
                x: self.x,
                y: self.y - 2,
                z: self.z,
            },
            Self {
                x: self.x,
                y: self.y,
                z: self.z + 2,
            },
            Self {
                x: self.x,
                y: self.y,
                z: self.z - 2,
            },
        ];

        (neighbors, second_neighbors)
    }
}

// A theoretical square patch of fabric
#[derive(Clone, Copy)]
pub struct ClothSegmentFrag {
    index: Index3,
    position: Vector3,
    pub velocity: Vector3,
    pinned: bool,
    pub link_vector: Option<f32>,
    pub rotation: Vector3,
}
pub struct ClothSegment {
    frag: ClothSegmentFrag,
    neighbors: Vec<Index3>,
    neighbor_index: Vec<usize>,
    second_neighbors: Vec<Option<Index3>>,
    second_neighbor_index: Vec<Option<usize>>,
    pub link_number: Option<u32>,
    pub index: usize,
}

pub struct Cloth {
    pub segments: Vec<ClothSegment>,
    scale: f32,
}

impl Cloth {
    pub fn generate_square(width: i32, height: i32, scale: f32) -> Self {
        let mut segments: Vec<ClothSegment> = vec![];
        let mut index = 0;
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
                        rotation: Vector3 {
                            x: 0.0,
                            y: 1.0,
                            z: 0.0,
                        },
                    },
                    neighbors: vec![],
                    neighbor_index: vec![],
                    second_neighbors: vec![],
                    second_neighbor_index: vec![],
                    link_number: None,
                    index,
                });
                index += 1;
            }
        }
        let mut ret = Cloth { segments, scale };
        for x in 0..width {
            for y in 0..height {
                let (segment, neighbors, second_neighbors) =
                    ret.find_index_info(Index3 { x, y: 0, z: y }).unwrap();
                segment.neighbors = neighbors.0;
                segment.neighbor_index = neighbors.1;
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
            let mut check = Vector2 { x, y: min_bound.y };
            let mut confirmed_lines: Vec<Line> = vec![];
            let mut y_step = 0;

            let mut flip_points: Vec<f32> = vec![];
            for line in &draft.lines {
                if line.in_slice(check, 0.0) {
                    match line.get_intersect_on_x(check) {
                        None => {}
                        Some(intersection_point) => {
                            flip_points.push(intersection_point);
                        }
                    }
                }
            }
            while check.y < max_bound.y {
                let mut intersections: u32 = 0;

                let mut pinned = false;
                let mut link_vector: Option<f32> = None;
                let mut link_number: Option<u32> = None;
                for line in &draft.lines {
                    if line.hitbox(check, detail) {
                        if line.pinned {
                            pinned = true;
                        }

                        if line.link.is_some() {
                            link_vector =
                                Some((line.p1 - check).length() / (line.p1 - line.p2).length());
                            link_number = line.link;
                        }
                    }
                    if line.p1.x == line.p2.x {
                        continue;
                    }
                }
                for flip_point in &flip_points {
                    if *flip_point > check.y {
                        intersections += 1;
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
                        rotation: Vector3 {
                            x: 0.0,
                            y: 1.0,
                            z: 0.0,
                        },
                    };
                    segments.push(ClothSegment {
                        frag,
                        neighbors: vec![],
                        neighbor_index: vec![],
                        second_neighbors: vec![],
                        second_neighbor_index: vec![],
                        link_number,
                        index: insert_index as usize,
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

                check.y += detail;
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
                    Some((segment, neighbors, second_neighbors)) => {
                        segment.neighbors = neighbors.0;
                        segment.neighbor_index = neighbors.1;
                        segment.second_neighbors = second_neighbors.0;
                        segment.second_neighbor_index = second_neighbors.1;

                        if segment.frag.link_vector.is_some() {
                            let number = segment.link_number.unwrap();
                            let mut min_dist = f32::INFINITY;
                            let mut segment_index: Option<usize> = None;
                            for index in segment_links[&number].clone() {
                                if index as usize == segment.index {
                                    continue;
                                }
                                let dist = (segment_frags[index as usize].link_vector.unwrap()
                                    - segment.frag.link_vector.unwrap())
                                .abs();

                                if dist < min_dist {
                                    min_dist = dist;
                                    segment_index = Some(index as usize);
                                }
                            }
                            match segment_index {
                                None => {}
                                Some(index) => {
                                    segment.neighbors.push(segment_frags[index as usize].index);
                                    segment.neighbor_index.push(index);
                                }
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
                } else if segment.link_number.is_some() {
                    color::Color::BLUE
                } else {
                    color::Color::GREEN
                },
            );
            r.draw_line_3D(
                segment.frag.position,
                segment.frag.position + segment.frag.rotation * self.scale * 2.0,
                color::Color::GAINSBORO,
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
                let dist = (frag.index - segment.frag.index).length();
                let change = self.scale * dist - diff.length();
                segment.frag.velocity += diff.normalized().scale_by(change * 0.5);
            }
            let mut index_index = 0;
            for index in &segment.second_neighbor_index {
                if !index.is_some() {
                    continue;
                }

                let far_frag = segment_memory[index.unwrap()];
                let center = segment_memory[segment.neighbor_index[index_index]];
                let targeted_pos: Vector3 = ((far_frag.position - center.position) * (-1.0 as f32));
                let movement_vec = (targeted_pos - (segment.frag.position - center.position));
                segment.frag.velocity +=
                    movement_vec.normalized() * (movement_vec.length() * 0.1).min(0.1);

                index_index += 1
            }

            // terminal velocity so things stabilize quicker
            let new_max = segment.frag.velocity.length().min(2.0);
            segment.frag.velocity = segment.frag.velocity.normalized() * new_max;

            if !segment.frag.pinned {
                segment.frag.position += segment.frag.velocity;
            }
        }
    }
    fn get_neighbors(
        &self,
        index: Index3,
    ) -> (
        (Vec<Index3>, Vec<usize>),
        (Vec<Option<Index3>>, Vec<Option<usize>>),
    ) {
        let mut ret: Vec<Index3> = vec![];
        let mut ret_index: Vec<usize> = vec![];
        let mut second_ret: Vec<Option<Index3>> = vec![];
        let mut second_ret_index: Vec<Option<usize>> = vec![];
        let (neighbors, second_neighbors) = index.get_neighbors();
        for neighbor in neighbors {
            let mut neighbor_index = 0;
            for segment in &self.segments {
                if segment.frag.index == neighbor {
                    ret.push(segment.frag.index);
                    ret_index.push(neighbor_index);
                }
                neighbor_index += 1;
            }
        }
        'optional_loop: for neighbor in &ret {
            let mut neighbor_index = 0;
            for segment in &self.segments {
                if segment.frag.index == *neighbor {
                    second_ret.push(Some(segment.frag.index));
                    second_ret_index.push(Some(neighbor_index));
                    continue 'optional_loop;
                }
                neighbor_index += 1;
            }

            second_ret.push(None);
            second_ret_index.push(None);
        }
        ((ret, ret_index), (second_ret, second_ret_index))
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
    ) -> Option<(
        &mut ClothSegment,
        (Vec<Index3>, Vec<usize>),
        (Vec<Option<Index3>>, Vec<Option<usize>>),
    )> {
        let (neighbors, second_neighbors) = self.get_neighbors(index);
        for segment in self.segments.iter_mut() {
            if segment.frag.index == index {
                return Some((segment, neighbors, second_neighbors));
            }
        }
        return None;
    }
}
