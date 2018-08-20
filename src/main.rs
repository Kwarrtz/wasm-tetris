#![feature(vec_remove_item)]
#![warn(rust_2018_idioms)]

use stdweb::{js,_js_impl,__js_raw_asm};
use stdweb::traits::IKeyboardEvent;
use stdweb::web::*;
use stdweb::web::event::KeyUpEvent;
use stdweb::web::html_element::CanvasElement;
use stdweb::unstable::TryInto;

use rand::*;

use std::sync::mpsc::*;

mod piece;
use crate::piece::*;

const CELL_SIZE: u32 = 30;
const COLS: u32 = 10;
const ROWS: u32 = 24;
const HIDDEN: u32 = 4;
const WIDTH: u32 = 300;
const HEIGHT: u32 = 600;
const WIDTH_AUX: u32 = 140;
const INIT_INTERVAL: u32 = 750;
const MIN_INTERVAL: u32 = 150;
const INTERVAL_INCR: u32 = 100;

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

enum State {
    GameOver(u32),
    Game {
        board: Vec<(u32,u32)>,
        active: Piece,
        next: Shape,
        score: u32,
        rng: ThreadRng,
        playing: bool,
        interval: u32,
    }
}

use self::State::*;

impl State {
    fn new_game() -> Self {
        let mut rng = thread_rng();
        Game {
            board: vec![],
            active: Piece::new(rng.gen(), &mut rng),
            next: rng.gen(),
            score: 0,
            rng: rng,
            playing: true,
            interval: INIT_INTERVAL,
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

        GameOver(_score) => {
            ctx.set_font("30pt Arial");
            ctx.set_text_align(TextAlign::Center);
            ctx.fill_text("GAME OVER", WIDTH as f64 / 2.0, HEIGHT as f64 / 2.0, None);
        }
    }
}

fn render_aux(state: &State, ctx: &mut CanvasRenderingContext2d) {
    ctx.set_fill_style_color("#000");
    ctx.fill_rect(0.0, 0.0, WIDTH.into(), HEIGHT.into());
    ctx.set_fill_style_color("#FFF");

    if let Game { next, score, .. } = state {
        ctx.set_font("35px Arial");
        ctx.set_text_align(TextAlign::Center);
        ctx.set_text_baseline(TextBaseline::Top);
        ctx.fill_text(&format!("Score: {}", score), WIDTH_AUX as f64 / 2.0, 10.0, None);

        let (t,l,_,r) = next.bounds();

        let correction = if r - l <= 2 { 1 } else { 0 };

        for s in next.pieces() {
            ctx.fill_rect(
                10.0 + ((s.0 - l + correction) * CELL_SIZE as i32) as f64,
                80.0 + ((s.1 - t) * CELL_SIZE as i32) as f64,
                CELL_SIZE as f64, CELL_SIZE as f64
            )
        }
    }
}

fn update(state: &mut State, event: &Event, s: Sender<Event>) {
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
        Game { board, active, rng, score, next, interval, playing: playing @ true } => {
            match event {
                Tick => {
                    active.center.1 += 1;

                    let tick = move || { s.send(Tick).expect("Could not send tick event"); };
                    js! { setTimeout(@{tick}, @{*interval}); }
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
                    *playing = false;
                }

                _ => { },
            }

            if active.squares().iter().any(
                |&(x,y)| board.contains(&(x, y+1)) || y >= ROWS - 1
            ) {
                board.append(&mut active.squares());
                *active = Piece::new(*next, rng);
                *next = rng.gen();

                for row in 0..ROWS {
                    if (0..COLS).all(|col| board.contains(&(col,row))) {
                        (0..COLS).for_each(|col| { board.remove_item(&(col,row)); });
                        for s in board.iter_mut() { if s.1 < row { s.1 += 1; } }

                        *score += 1;

                        if *interval > MIN_INTERVAL {
                            *interval -= INTERVAL_INCR;
                        }
                    }
                }

            }

            if board.iter().any(|&(_,y)| y <= HIDDEN) {
                *state = GameOver(*score);
            }
        }

        Game { playing: playing @ false, .. } => {
            if *event == Key("KeyP".into()) {
                *playing = true;
            }
        }

        GameOver(_) => {
            if *event == Key("Space".into()) {
                *state = State::new_game();
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

    let mut state = State::new_game();

    let (s1,r) = channel();
    let s2 = s1.clone();

    s1.send(Tick).expect("Could not send tick event");

    window().add_event_listener(move |e: KeyUpEvent| {
        s1.send(Key(e.code())).expect("Could not send keypress event");
    });

    js! {
        var callback = @{move || {
            if let Ok(event) = r.try_recv() {
                update(&mut state, &event, s2.clone());
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
