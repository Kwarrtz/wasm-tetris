use rand::distributions::{Standard,Distribution};
use rand::Rng;

#[derive(Clone,Copy)]
pub enum Genus { I, J, L, O, S, Z, T }

#[derive(Clone,Copy)]
pub enum Orientation { R0, R90, R180, R270 }
crate use self::Orientation::*;

#[derive(Clone,Copy)]
pub struct Shape {
    genus: Genus,
    orientation: Orientation,
}

impl Shape {
    pub fn pieces(&self) -> Vec<(i32,i32)> {
        use self::Genus::*;
        match self.genus {
            J => vec![(0,0),(0,-2),(0,-1),(-1,0)],
            L => vec![(0,0),(0,-2),(0,-1),(1,0)],
            T => vec![(0,0),(-1,0),(1,0),(0,1)],
            S => vec![(0,0),(-1,0),(0,1),(1,1)],
            Z => vec![(0,0),(1,0),(0,1),(-1,1)],
            I => vec![(0,0),(0,-1),(0,1),(0,2)],
            O => vec![(0,0),(1,0),(0,1),(1,1)]
        }.iter().map(|&(x,y)| {
            match self.orientation {
                R0 => (x,y),
                R90 => (-y,x),
                R180 => (-x,-y),
                R270 => (y,-x)
            }
        }).collect()
    }

    pub fn rotate(&mut self) {
        self.orientation = match self.orientation {
            R0 => R90,
            R90 => R180,
            R180 => R270,
            R270 => R0
        };
    }
}

impl Distribution<Orientation> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Orientation {
        match rng.gen_range(0,4) {
            0 => R0, 1 => R90, 2 => R180, _ => R270
        }
    }
}

impl Distribution<Shape> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Shape {
        Shape {
            genus: rng.gen(),
            orientation: rng.gen(),
        }
    }
}

impl Distribution<Genus> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Genus {
        use self::Genus::*;
        match rng.gen_range(0, 7) {
            0 => I, 1 => J, 2 => L, 3 => O, 4 => S, 5 => Z, _ => T
        }
    }
}
