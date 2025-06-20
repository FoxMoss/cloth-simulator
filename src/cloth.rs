use async_channel::{Receiver, Sender};
use raylib::prelude::*;
use std::ops::{Add, Sub};
use std::{collections::HashMap, f32, usize};

use crate::Message;
use crate::drafting::Draft;

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
impl Add for Index3 {
    type Output = Index3;
    fn add(self, v: Index3) -> Self {
        Index3 {
            x: self.x + v.x,
            y: self.y + v.y,
            z: self.z + v.z,
        }
    }
}
impl Index3 {
    pub fn length(&self) -> f32 {
        ((self.x.pow(2) + self.y.pow(2) + self.z.pow(2)) as f32).sqrt()
    }
}

impl Index3 {
    pub fn get_neighbors(&self, stiffness: u32) -> (Vec<Self>, Vec<Self>) {
        let mut neighbors: Vec<Self> = vec![];
        let scale: i32 = stiffness as i32;
        for x in (-scale)..=scale {
            for y in (-scale)..=scale {
                for z in (-scale)..=scale {
                    neighbors.push(Self {
                        x: self.x + x,
                        y: self.y + y,
                        z: self.z + z,
                    });
                }
            }
        }
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
    rigid: bool,
    pub link_vector: Option<f32>,
    pub link_number: Option<u32>,
    pub line_id: usize,
}
pub struct ClothSegment {
    frag: ClothSegmentFrag,
    neighbors: Vec<Index3>,
    neighbor_index: Vec<usize>,
    second_neighbors: Vec<Option<Index3>>,
    second_neighbor_index: Vec<Option<usize>>,
    pub index: usize,
}

pub struct Cloth {
    pub segments: Vec<ClothSegment>,
    sections: Vec<Vec<usize>>,
    scale: f32,

    pub gravity: f32,
    pub drag: f32,
    pub strength: f32,
    pub seam_strength: f32,
    pub stiffness: u32,

    quads: Vec<Vec<u32>>,
}

impl Cloth {
    pub fn generate_from_none() -> Self {
        Cloth {
            segments: vec![],
            sections: vec![],
            scale: 0.0,
            gravity: 0.0,
            drag: 0.0,
            strength: 0.0,
            seam_strength: 0.0,
            stiffness: 0,
            quads: vec![],
        }
    }
    pub fn generate_from_draft(
        draft: &Draft,
        scale: f32,
        stiffness: u32,
        detail: f32,
        sender: &Sender<Message>,
        receiver: &Receiver<Message>,
        gravity: f32,
        drag: f32,
        strength: f32,
        seam_strength: f32,
    ) -> Option<Self> {
        sender
            .send_blocking(Message::RenderProgress(0.0))
            .expect("The channel needs to be open.");

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
            sender
                .send_blocking(Message::RenderProgress(
                    (((x - min_bound.x) / (max_bound.x - min_bound.x)) / 3.0) as f64,
                ))
                .expect("The channel needs to be open.");
            let mut check = Vector2 { x, y: min_bound.y };
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
                let mut rigid = false;
                let mut link_vector: Option<f32> = None;
                let mut link_number: Option<u32> = None;
                let mut line_id: usize = 0;
                for line in &draft.lines {
                    if line.hitbox(check, detail * 1.5) {
                        if line.pinned {
                            pinned = true;
                        }

                        rigid = line.rigid;

                        if line.link.is_some() {
                            let mut true_p1 = line.p1;
                            let mut true_p2 = line.p2;

                            // lower point should be p2 to avoid linking issues
                            if line.p1.x < line.p2.x
                                || (line.p1.x == line.p2.x && line.p1.y < line.p1.y)
                            {
                                true_p1 = line.p2;
                                true_p2 = line.p1;
                            }

                            link_vector =
                                Some((true_p1 - check).length() / (true_p1 - true_p2).length());
                            link_number = line.link;
                            line_id = line.line_id;
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
                        rigid,
                        link_vector,
                        link_number,
                        line_id,
                    };
                    segments.push(ClothSegment {
                        frag,
                        neighbors: vec![],
                        neighbor_index: vec![],
                        second_neighbors: vec![],
                        second_neighbor_index: vec![],
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

        let mut ret = Cloth {
            segments,
            sections: vec![],
            scale,
            gravity,
            drag,
            strength,
            seam_strength,
            stiffness,
            quads: vec![],
        };

        let mut sections: Vec<Vec<usize>> = vec![];
        let mut total_so_far = 0;
        'sectional: for segment in &ret.segments {
            for section in &sections {
                if section.contains(&segment.index) {
                    continue 'sectional;
                }
            }
            sections.push(vec![]);
            let section_res = ret.discover_section(
                segment.index,
                sections,
                sender,
                receiver,
                ret.segments.len(),
                total_so_far,
            );
            if section_res.is_none() {
                return None;
            }
            sections = section_res.unwrap();
            total_so_far += sections.last().unwrap().len();
        }
        ret.sections = sections;

        for x in 0..x_index_max {
            let cancel_check = receiver.try_recv();
            match cancel_check {
                Ok(Message::Back) => {
                    return None;
                }
                _ => {}
            }
            sender
                .send_blocking(Message::RenderProgress(
                    ((x as f32 / x_index_max as f32) / 3.0 + 2.0 / 3.0) as f64,
                ))
                .expect("The channel needs to be open.");
            for y in 0..y_index_max {
                match ret.find_index_info(Index3 { x, y: 0, z: y }) {
                    None => {}
                    Some((segment, neighbors, second_neighbors)) => {
                        segment.neighbors = neighbors.0;
                        segment.neighbor_index = neighbors.1;
                        segment.second_neighbors = second_neighbors.0;
                        segment.second_neighbor_index = second_neighbors.1;

                        if segment.frag.link_vector.is_some() {
                            let number = segment.frag.link_number.unwrap();
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

        let mut segment_memory: Vec<ClothSegmentFrag> = vec![];
        for segment in &ret.segments {
            segment_memory.push(segment.frag);
        }

        for segment in &ret.segments {
            // r.draw_cube(
            //     segment.frag.position,
            //     self.scale,
            //     self.scale,
            //     self.scale,
            //     if segment.frag.pinned {
            //         color::Color::RED
            //     } else if segment.frag.link_number.is_some() {
            //         color::Color::BLUE
            //     } else {
            //         color::Color::GREEN
            //     },
            // );
            for x_dir in [-1, 1] {
                for y_dir in [-1, 1] {
                    let mut quad: Vec<u32> = vec![];
                    for index in &segment.neighbor_index {
                        let frag = segment_memory[*index];
                        let a_value: i32 = frag.index.x - segment.frag.index.x;
                        let b_value: i32 = frag.index.z - segment.frag.index.z;
                        if a_value.abs() > 1 || b_value.abs() > 1 {
                            continue;
                        }
                        if a_value / x_dir >= 0 && b_value / y_dir >= 0 {
                            quad.push(*index as u32);
                        }
                    }
                    if quad.len() == 4 {
                        ret.quads.push(quad);
                    }
                }
            }

            // r.draw_line_3D(
            //     segment.frag.position,
            //     segment.frag.position + segment.frag.velocity,
            //     color::Color::RED,
            // );
        }

        sender
            .send_blocking(Message::RenderProgress(1.0))
            .expect("The channel needs to be open.");

        Some(ret)
    }

    pub fn discover_section(
        &self,
        index: usize,
        current_section: Vec<Vec<usize>>,
        sender: &Sender<Message>,
        receiver: &Receiver<Message>,
        length: usize,
        total: usize,
    ) -> Option<Vec<Vec<usize>>> {
        let mut current_section_copy = current_section;
        let last_index_pos = current_section_copy.len() - 1;
        current_section_copy[last_index_pos].push(index);
        let cancel_check = receiver.try_recv();
        match cancel_check {
            Ok(Message::Back) => {
                return None;
            }
            _ => {}
        }

        sender
            .send_blocking(Message::RenderProgress(
                (1.0 / 3.0) + ((current_section_copy.len() + total) as f64 / length as f64) / 3.0,
            ))
            .expect("The channel needs to be open.");

        let selected_index = self.segments[index].frag.index;
        'segment_search: for segment in &self.segments {
            for section in &current_section_copy {
                if section.contains(&segment.index) {
                    continue 'segment_search;
                }
            }

            for x in -1..=1 {
                for y in -1..=1 {
                    for z in -1..=1 {
                        let searching_index = selected_index + Index3 { x, y, z };
                        if segment.frag.index == searching_index {
                            let current_section_copy_res = self.discover_section(
                                segment.index,
                                current_section_copy,
                                sender,
                                receiver,
                                length,
                                total,
                            );
                            if current_section_copy_res.is_none() {
                                return None;
                            }
                            current_section_copy = current_section_copy_res.unwrap();
                        }
                    }
                }
            }
        }
        return Some(current_section_copy);
    }

    pub fn draw(&self, r: &mut RaylibMode3D<'_, RaylibDrawHandle<'_>>) {
        let mut segment_memory: Vec<ClothSegmentFrag> = vec![];
        for segment in &self.segments {
            segment_memory.push(segment.frag);
        }

        for quad in &self.quads {
            let mut last_index = *quad.last().unwrap();
            let mut traveled: Vec<u32> = vec![last_index];
            while traveled.len() != 4 {
                for index in quad {
                    if traveled.contains(&index) {
                        continue;
                    }

                    let index_pos = segment_memory[*index as usize].index;
                    let last_index_pos = segment_memory[last_index as usize].index;

                    if (index_pos.x - last_index_pos.x).abs()
                        + (index_pos.z - last_index_pos.z).abs()
                        != 1
                    {
                        continue;
                    }

                    r.draw_line_3D(
                        segment_memory[last_index as usize].position,
                        segment_memory[*index as usize].position,
                        if segment_memory[*index as usize].rigid {
                            color::Color::ORANGE
                        } else if segment_memory[*index as usize].pinned {
                            color::Color::RED
                        } else if segment_memory[*index as usize].link_number.is_some() {
                            color::Color::BLUE
                        } else {
                            color::Color::GREEN
                        },
                    );
                    traveled.push(*index);
                    last_index = *index;
                }
            }
            r.draw_line_3D(
                segment_memory[*traveled.first().unwrap() as usize].position,
                segment_memory[*traveled.last().unwrap() as usize].position,
                if segment_memory[*traveled.first().unwrap() as usize].rigid {
                    color::Color::ORANGE
                } else if segment_memory[*traveled.first().unwrap() as usize].pinned {
                    color::Color::RED
                } else if segment_memory[*traveled.first().unwrap() as usize]
                    .link_number
                    .is_some()
                {
                    color::Color::BLUE
                } else {
                    color::Color::GREEN
                },
            );
        }
    }
    pub fn step(&mut self) {
        let mut rigid_plane = 0.0;
        let mut rigid_len = 0;
        let mut segment_memory: Vec<ClothSegmentFrag> = vec![];
        for segment in &self.segments {
            if segment.frag.rigid {
                rigid_plane += segment.frag.position.y;
                rigid_len += 1;
            }
            segment_memory.push(segment.frag);
        }
        rigid_plane /= rigid_len as f32;

        for segment in self.segments.iter_mut() {
            segment.frag.velocity += Vector3 {
                x: 0.0,
                y: -self.gravity,
                z: 0.0,
            };

            let mut neighbor_forces = Vector3::zero();
            let mut segment_section: Vec<usize> = vec![];
            for section in &self.sections {
                if section.contains(&segment.index) {
                    segment_section = section.clone();
                }
            }

            for index in &segment.neighbor_index {
                if *index == segment.index {
                    continue;
                }
                let frag = segment_memory[*index];

                let diff = frag.position - segment.frag.position;
                let mut dist = (frag.index - segment.frag.index).length();
                let mut mult = self.strength;

                if segment.frag.pinned {
                    mult = self.seam_strength;
                }
                if !segment_section.contains(index) {
                    mult = 0.0;
                }

                // there should be a better way
                if segment
                    .frag
                    .link_number
                    .is_some_and(|a| frag.link_number.is_some_and(|b| a == b))
                    && frag.line_id != segment.frag.line_id
                {
                    dist = 0.0;
                    mult = self.seam_strength;
                }

                let change = (self.scale * dist) - diff.length();
                let scaled = diff.normalized().scale_by(-change);
                neighbor_forces += scaled.scale_by(mult);
            }
            if segment.frag.rigid {
                let set_pos = Vector3 {
                    x: 0.0,
                    y: rigid_plane - segment.frag.position.y,
                    z: 0.0,
                };
                neighbor_forces += set_pos * 0.3;
            }

            segment.frag.velocity += neighbor_forces;

            segment.frag.velocity *= self.drag;

            if !segment.frag.pinned {
                segment.frag.position += segment.frag.velocity;
            }
        }
    }
    fn get_neighbors(
        &self,
        index: Index3,
        stiffness: u32,
    ) -> (
        (Vec<Index3>, Vec<usize>),
        (Vec<Option<Index3>>, Vec<Option<usize>>),
    ) {
        let mut ret: Vec<Index3> = vec![];
        let mut ret_index: Vec<usize> = vec![];
        let mut second_ret: Vec<Option<Index3>> = vec![];
        let mut second_ret_index: Vec<Option<usize>> = vec![];
        let (neighbors, second_neighbors) = index.get_neighbors(stiffness);
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

    fn find_index_info(
        &mut self,
        index: Index3,
    ) -> Option<(
        &mut ClothSegment,
        (Vec<Index3>, Vec<usize>),
        (Vec<Option<Index3>>, Vec<Option<usize>>),
    )> {
        let (neighbors, second_neighbors) = self.get_neighbors(index, self.stiffness);
        for segment in self.segments.iter_mut() {
            if segment.frag.index == index {
                return Some((segment, neighbors, second_neighbors));
            }
        }
        return None;
    }
}
