#![feature(vec_remove_item)]
#![warn(rust_2018_idioms)]

use stdweb::{js,_js_impl,__js_raw_asm};
use stdweb::traits::IKeyboardEvent;
use stdweb::web::*;
use stdweb::web::event::KeyUpEvent;
use stdweb::web::html_element::CanvasElement;
use stdweb::unstable::TryInto;

use rand::*;
use rand::distributions::{Standard,Distribution};

use std::sync::mpsc::*;

use std::cmp::{min,max};

const CELL_SIZE: u32 = 30;
const COLS: u32 = 10;
const ROWS: u32 = 24;
const HIDDEN: u32 = 4;
const WIDTH: u32 = 300;
const HEIGHT: u32 = 600;
const INIT_INTERVAL: u32 = 750;
const MIN_INTERVAL: u32 = 150;
const INTERVAL_INCR: u32 = 100;

enum State {
    GameOver {
        score: u32,
        beat_highscore: bool,
    },
    Game {
        board: Vec<(u32,u32)>,
        active: Piece,
        next: Shape,
        score: u32,
        highscore: Option<u32>,
        rng: ThreadRng,
        playing: bool,
        interval: u32,
        next_tick_id: u32,
    }
}

use self::State::*;

impl State {
    fn new_game(s: Sender<Event>) -> Self {
        let mut rng = thread_rng();
        let id = schedule_tick(s, INIT_INTERVAL);
        Game {
            board: vec![],
            active: Piece::new(rng.gen(), &mut rng),
            next: rng.gen(),
            score: 0,
            highscore: get_highscore_cookie(),
            rng: rng,
            playing: true,
            interval: INIT_INTERVAL,
            next_tick_id: id,
        }
    }
}

#[derive(PartialEq)]
enum Event { Key(String), Tick }
use self::Event::*;

fn render_main(state: &State, ctx: &mut CanvasRenderingContext2d) {
    ctx.set_fill_style_color("#000");
    ctx.fill_rect(0.0, 0.0, WIDTH.into(), HEIGHT.into());
    ctx.set_fill_style_color("#FFF");

    match state {
        Game { board, active, .. } => {
            for s in [&board[..], &active.squares()[..]].concat() {
                if s.0 < COLS && s.1 < ROWS {
                    ctx.fill_rect(
                        (s.0 * CELL_SIZE) as f64, ((s.1 - HIDDEN) * CELL_SIZE) as f64,
                        CELL_SIZE as f64, CELL_SIZE as f64);
                }
            }
        }

        GameOver { score, beat_highscore } => {
            ctx.set_text_align(TextAlign::Center);
            ctx.set_font("30pt Arial");
            ctx.set_text_baseline(TextBaseline::Bottom);

            ctx.fill_text("GAME OVER",
                WIDTH as f64 / 2.0, HEIGHT as f64 / 2.0, None);

            ctx.set_font("25px Arial");
            ctx.set_text_baseline(TextBaseline::Top);

            ctx.fill_text(&format!("Final Score: {}", score),
                WIDTH as f64 / 2.0, HEIGHT as f64 / 2.0 + 20.0, None);

            if *beat_highscore {
                ctx.fill_text("NEW HIGHSCORE",
                    WIDTH as f64 / 2.0, HEIGHT as f64 / 2.0 + 60.0, None);
            }
        }
    }
}

fn render_aux(state: &State, ctx: &mut CanvasRenderingContext2d) {
    ctx.set_fill_style_color("#000");
    ctx.fill_rect(0.0, 0.0, WIDTH.into(), HEIGHT.into());
    ctx.set_fill_style_color("#FFF");

    match state {
        Game { next, score, highscore, .. } => {
            ctx.set_font("35px Arial");
            ctx.set_text_align(TextAlign::Left);
            ctx.set_text_baseline(TextBaseline::Top);
            ctx.fill_text("Up Next:", 0.0, 10.0, None);

            let (t,l,_,r) = next.bounds();

            let correction = if r - l <= 2 { 1 } else { 0 };

            for s in next.pieces() {
                ctx.fill_rect(
                    10.0 + ((s.0 - l + correction) * CELL_SIZE as i32) as f64,
                    80.0 + ((s.1 - t) * CELL_SIZE as i32) as f64,
                    CELL_SIZE as f64, CELL_SIZE as f64
                )
            }

            ctx.set_font("25px Arial");

            ctx.fill_text(&format!("Score: {}", score), 0.0, 230.0, None);

            if let Some(hs) = highscore {
                ctx.fill_text(&format!("Highscore: {}", hs), 0.0, 265.0, None);
            }
        },
        _ => { }
    }
}

fn update(state: &mut State, event: &Event, s: Sender<Event>) {
    // intercept 'M' key to pause/play music
    if *event == Key("KeyM".to_string()) {
        js! {
            var audio = document.getElementById("soundtrack");
            if (audio.paused) {
                audio.play();
            } else {
                audio.pause();
            }
        }

        return
    }

    match state {
        Game { board, active, rng, score, highscore, next, interval, next_tick_id, playing: playing @ true } => {
            match event {
                Tick => {
                    active.center.1 += 1;

                    *next_tick_id = schedule_tick(s, *interval)
                }

                Key(ref c) if c == "ArrowLeft" || c == "KeyA" => {
                    if !active.squares().iter().any(
                        |s| board.contains(&(s.0-1,s.1)) || s.0 <= 0
                    ) {
                        active.center.0 -= 1;
                    }
                }

                Key(ref c) if c == "ArrowRight" || c == "KeyD" => {
                    if !active.squares().iter().any(
                        |s| board.contains(&(s.0+1,s.1)) || s.0 >= COLS - 1
                    ) {
                        active.center.0 += 1;
                    }
                }

                Key(ref c) if c == "ArrowUp" || c == "KeyW" => {
                    let mut rotated = active.clone();
                    rotated.shape.rotate();
                    if !rotated.squares().iter().any(
                        |s| board.contains(s) || s.0 > COLS - 1
                    ) {
                        *active = rotated;
                    }
                }

                Key(ref c) if c == "ArrowDown" || c == "KeyS" => {
                    active.center.1 += 1;
                }

                Key(ref c) if c == "Space" => {
                    while !active.squares().iter().any(
                        |s| board.contains(&(s.0,s.1+1)) || s.1 >= ROWS - 1
                    ) {
                        active.center.1 += 1;
                    }
                }

                Key(ref c) if c == "KeyP" => {
                    // playing = true already matched, so pause
                    *playing = false;
                }

                _ => { },
            }

            // if active piece makes contact with floor
            if active.squares().iter().any(
                |&(x,y)| board.contains(&(x, y+1)) || y >= ROWS - 1
            ) {
                board.append(&mut active.squares());
                *active = Piece::new(*next, rng);
                *next = rng.gen();

                for row in 0..ROWS {
                    // if row is complete
                    if (0..COLS).all(|col| board.contains(&(col,row))) {
                        (0..COLS).for_each(|col| { board.remove_item(&(col,row)); }); // remove row
                        for s in board.iter_mut() { if s.1 < row { s.1 += 1; } } // shift rows above

                        *score += 1;

                        // speed up blocks (unless already at max speed)
                        *interval = max(*interval - INTERVAL_INCR, MIN_INTERVAL);
                    }
                }

            }

            // if board is full (game over)
            if board.iter().any(|&(_,y)| y <= HIDDEN) {
                let (beat, new_highscore) = match highscore {
                    Some(old_highscore) if *score <= *old_highscore => (false, *old_highscore),
                    _ => (true, *score)
                };

                js! { document.cookie = "highscore=" + @{new_highscore}; }

                // clear in case user starts new game before pending tick goes through
                // (results in blocks jumping twice in next game)
                js! { clearTimeout(@{*next_tick_id}); }

                *state = GameOver { score: *score, beat_highscore: beat };
            }
        }

        Game { playing: playing @ false, .. } => {
            // resume
            if *event == Key("KeyP".into()) {
                *playing = true;
            }
        }

        GameOver { .. } => {
            // new game
            if *event == Key("Space".into()) {
                *state = State::new_game(s);
            }
        }
    }
}

fn main() {
    stdweb::initialize();

    let canvas_main: CanvasElement = document().get_element_by_id("main").unwrap().try_into().unwrap();
    let canvas_aux: CanvasElement = document().get_element_by_id("aux").unwrap().try_into().unwrap();

    let mut ctx_main: CanvasRenderingContext2d = canvas_main.get_context().unwrap();
    let mut ctx_aux: CanvasRenderingContext2d = canvas_aux.get_context().unwrap();

    let (s,r) = channel();

    let mut state = State::new_game(s.clone());

    let s_key = s.clone();
    window().add_event_listener(move |e: KeyUpEvent| {
        s_key.send(Key(e.code())).expect("Could not send keypress event");
    });

    // continuously poll for new events on channel
    js! {
        var callback = @{move || {
            if let Ok(event) = r.try_recv() {
                update(&mut state, &event, s.clone());
                render_main(&state, &mut ctx_main);
                render_aux(&state, &mut ctx_aux);
            };
        }};

        function loop() {
            callback();
            requestAnimationFrame(loop);
        }

        requestAnimationFrame(loop);
    };
}

fn schedule_tick(s: Sender<Event>, interval: u32) -> u32 {
    let tick = move || { s.send(Tick).expect("Could not send tick event"); };
    let id: u32 = js!( return setTimeout(@{tick}, @{interval}); ).try_into().unwrap();
    id
}

fn get_highscore_cookie() -> Option<u32> {
    let cookie: String = js!( return document.cookie; ).try_into().unwrap();
    let mut cookie_iter = cookie.split('=');
    cookie_iter.next()?;
    cookie_iter.next()?.parse().ok()
}

#[derive(Clone)]
struct Piece {
    center: (u32,u32),
    shape: Shape,
}

impl Piece {
    fn new(shape: Shape, rng: &mut impl Rng) -> Piece {
        let (_,l,b,r) = shape.bounds();

        Piece {
            center: ( rng.gen_range(-l as u32, COLS - r as u32), HIDDEN - b as u32),
            shape: shape,
        }
    }

    fn squares(&self) -> Vec<(u32,u32)> {
        self.shape.pieces().iter().map(|s| {
            let x = s.0 + self.center.0 as i32;
            let y = s.1 + self.center.1 as i32;
            (x as u32, y as u32)
        }).collect()
    }
}

#[derive(Clone,Copy)]
enum Genus { I, J, L, O, S, Z, T }

#[derive(Clone,Copy)]
enum Orientation { R0, R90, R180, R270 }
use self::Orientation::*;

#[derive(Clone,Copy)]
struct Shape {
    genus: Genus,
    orientation: Orientation,
}

impl Shape {
    fn pieces(&self) -> Vec<(i32,i32)> {
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

    fn rotate(&mut self) {
        self.orientation = match self.orientation {
            R0 => R90,
            R90 => R180,
            R180 => R270,
            R270 => R0
        };
    }

    fn bounds(&self) -> (i32,i32,i32,i32) {
        let (mut t, mut l, mut b, mut r) = (0,0,0,0);
        for s in self.pieces() {
            t = min(t, s.1);
            l = min(l, s.0);
            b = max(b, s.1 + 1);
            r = max(r, s.0 + 1);
        }

        (t,l,b,r)
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
