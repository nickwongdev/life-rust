use std::collections::{BTreeMap, HashSet};
use std::fmt::{Display, Formatter};
use std::io;
use std::io::BufRead;
use std::ops::Bound::Included;
use std::process::exit;
use std::str::FromStr;
use std::sync::RwLock;

trait TimeBasedEntity {
    fn tick(&mut self);
}

#[derive(Debug, Copy, Clone)]
struct Life {
    x_pos:i64,
    y_pos:i64,
    age:u32
}

impl Life {
    pub const fn new(x_pos: i64, y_pos: i64) -> Life {
        Life { x_pos, y_pos, age: 0 }
    }

    pub fn is_close_neighbor(&self, neighbor: &Life) -> bool {
        let dist_x = neighbor.x_pos - self.x_pos;
        let dist_y = neighbor.y_pos - self.y_pos;
        dist_x >= -1 && dist_x <= 1 && dist_y >= -1 && dist_y <= 1
    }

    fn calculate_neighbor_coordinates(&self, pos: u8) -> Option<(i64, i64)> {
        match pos {
            0 => Some((i64::saturating_sub(self.x_pos, 1), i64::saturating_add(self.y_pos, 1))),
            1 => Some((self.x_pos, i64::saturating_add(self.y_pos, 1))),
            2 => Some((i64::saturating_add(self.x_pos, 1), i64::saturating_add(self.y_pos, 1))),
            3 => Some((i64::saturating_sub(self.x_pos, 1), self.y_pos)),
            4 => Some((i64::saturating_add(self.x_pos, 1), self.y_pos)),
            5 => Some((i64::saturating_sub(self.x_pos, 1), i64::saturating_sub(self.y_pos, 1))),
            6 => Some((self.x_pos, i64::saturating_sub(self.y_pos, 1))),
            7 => Some((i64::saturating_add(self.x_pos, 1), i64::saturating_sub(self.y_pos, 1))),
            _ => None
        }
    }
}

struct World {
    map:RwLock<BTreeMap<i64, BTreeMap<i64, Life>>>,
    age:u32
}

impl Display for World {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let x_map = self.map.read().unwrap();
        for (_x_key, x_value) in x_map.iter() {
            for (_y_key, life) in x_value.iter() {
                match writeln!(f, "{} {}", life.x_pos, life.y_pos) {
                    Ok(_) => {},
                    Err(_) => {}
                }
            }
        }
        Ok(())
    }
}

impl World {

    pub fn add_life(&mut self, life: Life) {
        let mut x_map = self.map.write().unwrap();
        match x_map.get_mut(&life.x_pos) {
            Some(y_map) => {
                if !y_map.contains_key(&life.y_pos) {
                    y_map.insert(life.y_pos.clone(), life);
                }
            },
            None => {
                let mut y_map: BTreeMap<i64, Life> = BTreeMap::new();
                let x_pos = life.x_pos.clone();
                y_map.insert(life.y_pos.clone(), life);
                x_map.insert(x_pos, y_map);
            }
        }
    }

    pub fn remove_life(&mut self, life: &Life) {
        let mut x_map = self.map.write().unwrap();
        match x_map.get_mut(&life.x_pos) {
            Some(y_map) => {
                if y_map.contains_key(&life.y_pos) {
                    y_map.remove(&life.y_pos);
                }
            },
            None => { }
        }
    }

    pub fn spatial_query(&self, start_x:i64, start_y:i64, end_x:i64, end_y:i64) -> Vec<Life> {
        let mut results: Vec<Life> = Vec::new();
        let x_map = self.map.read().unwrap();
        let x_range = x_map.range((Included(start_x), Included(end_x)));
        for (_x_key, x_value) in x_range {
            for (_y_key, life) in x_value.range((Included(end_y), Included(start_y))) {
                results.push(life.clone());
            }
        }
        return results;
    }

    fn query_around_point(&self, x: i64, y: i64) -> Vec<Life> {
        return self.spatial_query(
            i64::saturating_sub(x, 2),
            i64::saturating_add(y, 2),
            i64::saturating_add(x, 2),
            i64::saturating_sub(y, 2));
    }

    pub fn initialize(&self) {
        let mut x_map = self.map.write().unwrap();
        for (_x_key, x_value) in x_map.iter_mut() {
            for (_y_key, y_value) in x_value.iter_mut() {
                y_value.tick();
            }
        }
    }
}

impl TimeBasedEntity for Life {
    fn tick(&mut self) {
        self.age += 1;
    }
}

impl TimeBasedEntity for World {
    fn tick(&mut self) {
        let mut new_life_counters:[u8; 8];

        let mut kill_vec: Vec<Life> = Vec::new();
        let mut new_life_set: HashSet<(i64, i64)> = HashSet::new();

        {
            let x_map = self.map.read().unwrap();
            for (_x_index, y_map) in x_map.iter() {
                for (_y_index, life) in y_map.iter() {

                    // Skip Newborns
                    if life.age == 0 {
                        continue;
                    }

                    let mut close_neighbor_count: u8 = 0;

                    // Initialize to 1 to account for self
                    new_life_counters = [1; 8];

                    for neighbor in self.query_around_point(life.x_pos, life.y_pos) {
                        // Skip Self
                        if life.x_pos == neighbor.x_pos && life.y_pos == neighbor.y_pos {
                            continue;
                        }
                        // Skip newborns
                        if neighbor.age == 0 {
                            continue;
                        }

                        if life.is_close_neighbor(&neighbor) {
                            close_neighbor_count += 1;
                        }

                        update_new_life_counters(&mut new_life_counters, life, &neighbor);
                    }

                    for (i, counter) in new_life_counters.iter().enumerate() {
                        if *counter == 3 {
                            match life.calculate_neighbor_coordinates(i as u8) {
                                Some(coords) => new_life_set.insert(coords),
                                None => false
                            };
                        }
                    }

                    if !(close_neighbor_count == 2 || close_neighbor_count == 3) {
                        kill_vec.push(life.clone());
                    }
                }
            }
        }

        for life in kill_vec {
            self.remove_life(&life);
        }

        for coords in new_life_set {
            self.add_life(Life::new(coords.0, coords.1));
        }

        let mut x_map = self.map.write().unwrap();
        for (_x_index, y_map) in x_map.iter_mut() {
            for (_y_index, life) in y_map.iter_mut() {
                life.tick();
            }
        }

        self.age += 1;
    }
}

fn main() {
    let mut world: World = World { map: RwLock::new(BTreeMap::new()), age: 0 };

    let mut lineno = 0;
    for line_result in io::stdin().lock().lines() {
        match line_result {
            Ok(line) => {
                let clean_line = line.trim();
                if lineno == 0 {
                    if !clean_line.eq("#Life 1.06") {
                        println!("File is not a valid Life 1.06 file, does not begin with proper header");
                        exit(0)
                    } else {
                        lineno += 1;
                        continue;
                    }
                }
                let mut parts: Vec<&str> = clean_line.split(" ").collect();
                let y_str = parts.pop().unwrap();
                let x_str = parts.pop().unwrap();
                let x_pos = i64::from_str(x_str).unwrap();
                let y_pos = i64::from_str(y_str).unwrap();

                world.add_life(Life::new(x_pos, y_pos));
            }
            Err(_) => {
                exit(0)
            }
        }
    }

    world.initialize();

    for _ in 0..10 {
        world.tick();
    }

    println!("#Life 1.06");
    println!("{}", world);
}

fn update_new_life_counters(counters: &mut [u8; 8], center: &Life, neighbor: &Life) {
    let dist_x = neighbor.x_pos - center.x_pos;
    let dist_y = neighbor.y_pos - center.y_pos;

    match dist_y {
        2 => {
            match dist_x {
                -2 => counters[0] += 1,
                -1 => { counters[0] += 1; counters[1] += 1; }
                0 => { counters[0] += 1; counters[1] += 1; counters[2] += 1; }
                1 => { counters[1] += 1; counters[2] += 1; }
                2 => counters[2] += 1,
                _ => {}
            }
        }
        1 => {
            match dist_x {
                -2 => { counters[0] += 1; counters[3] += 1; }
                -1 => { counters[1] += 1; counters[3] += 1; }
                0 => { counters[0] += 1; counters[2] += 1; counters[3] += 1; counters[4] += 1; }
                1 => { counters[1] += 1; counters[4] += 1; }
                2 => { counters[2] += 1; counters[4] += 1; }
                _ => {}
            }
        }
        0 => {
            match dist_x {
                -2 => { counters[0] += 1; counters[3] += 1; counters[5] += 1; }
                -1 => { counters[0] += 1; counters[1] += 1; counters[5] += 1; counters[6] += 1; }
                1 => { counters[1] += 1; counters[2] += 1; counters[6] += 1; counters[7] += 1; }
                2 => { counters[2] += 1; counters[4] += 1; counters[7] += 1; }
                _ => {}
            }
        }
        -1 => {
            match dist_x {
                -2 => { counters[3] += 1; counters[5] += 1; }
                -1 => { counters[3] += 1; counters[6] += 1; }
                0 => { counters[3] += 1; counters[4] += 1; counters[5] += 1; counters[7] += 1; }
                1 => { counters[4] += 1; counters[6] += 1; }
                2 => { counters[4] += 1; counters[7] += 1; }
                _ => {}
            }
        }
        -2 => {
            match dist_x {
                -2 => { counters[5] += 1; }
                -1 => { counters[5] += 1; counters[6] += 1; }
                0 => { counters[5] += 1; counters[6] += 1; counters[7] += 1; }
                1 => { counters[6] += 1; counters[7] += 1; }
                2 => { counters[7] += 1; }
                _ => {}
            }
        }
        _ => {}
    }
}
