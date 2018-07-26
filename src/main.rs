#![feature(rust_2018_preview)]
#![feature(vec_remove_item,nll)]

#[macro_use]
extern crate stdweb;
use std::cmp::{max,min};
use stdweb::traits::IKeyboardEvent;
use stdweb::web::*;
use stdweb::web::event::KeyUpEvent;
use stdweb::web::html_element::CanvasElement;
use stdweb::unstable::TryInto;

extern crate rand;
use rand::*;

use std::sync::mpsc::*;

mod piece;
use piece::*;

const CANVAS_STYLE: &str = "
    border: solid #FFF;
    padding-left: 0;
    padding-right: 0;
    margin-top: 5em;
    margin-left: auto;
    margin-right: auto;
    display: block;
";

const CELL_SIZE: u32 = 30;
const COLS: u32 = 10;
const ROWS: u32 = 24;
const HIDDEN: u32 = 4;
const WIDTH: u32 = COLS * CELL_SIZE;
const HEIGHT: u32 = (ROWS - HIDDEN) * CELL_SIZE;

#[derive(Clone)]
struct Piece {
    center: (u32,u32),
    shape: Shape,
}

impl Piece {
    fn new<R: Rng + ?Sized>(rng: &mut R) -> Piece {
        let shape = rng.gen::<Shape>();

        let mut extents = (0,0,0);
        for s in shape.pieces() {
            extents.0 = min(extents.0, s.0);
            extents.1 = max(extents.1, s.1);
            extents.2 = max(extents.2, s.0 + 1);
        }

        Piece {
            center: ( rng.gen_range(-extents.0 as u32, COLS - extents.2 as u32)
                    , HIDDEN - 1 - extents.1 as u32),
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
        score: u32,
        rng: ThreadRng,
        playing: bool,
    }
}

use State::*;

impl State {
    fn new_game() -> Self {
        let mut rng = thread_rng();
        Game {
            board: vec![],
            active: Piece::new(&mut rng),
            score: 0,
            rng: rng,
            playing: true,
        }
    }
}

#[derive(PartialEq)]
enum Event { Key(String), Tick }
use Event::*;

fn render(state: &State, ctx: &mut CanvasRenderingContext2d) {
    ctx.set_fill_style_color("#000");
    ctx.fill_rect(0.0, 0.0, (COLS * CELL_SIZE) as f64, (ROWS * CELL_SIZE) as f64);
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

fn update(state: &mut State, event: Event) {
    match state {
        Game { board, active, rng, score, playing: playing @ true } => {
            match event {
                Tick => {
                    active.center.1 += 1;
                }

                Key(ref c) if c == "ArrowLeft" => {
                    if !active.squares().iter().any(
                        |s| board.contains(&(s.0-1,s.1)) || s.0 <= 0
                    ) {
                        active.center.0 -= 1;
                    }
                }

                Key(ref c) if c == "ArrowRight" => {
                    if !active.squares().iter().any(
                        |s| board.contains(&(s.0+1,s.1)) || s.0 >= COLS - 1
                    ) {
                        active.center.0 += 1;
                    }
                }

                Key(ref c) if c == "ArrowUp" => {
                    let mut rotated = active.clone();
                    rotated.shape.rotate();
                    if !rotated.squares().iter().any(
                        |s| board.contains(s) || s.0 <= 0 || s.0 >= COLS - 1
                    ) {
                        *active = rotated;
                    }
                }

                Key(ref c) if c == "ArrowDown" => {
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
                *active = Piece::new(rng);
                for row in 0..ROWS {
                    if (0..COLS).all(|col| board.contains(&(col,row))) {
                        (0..COLS).for_each(|col| { board.remove_item(&(col,row)); });
                        for s in board.iter_mut() { if s.1 < row { s.1 += 1; } }
                        *score += 1;
                    }
                }
            }

            if board.iter().any(|&(_,y)| y <= HIDDEN) {
                *state = GameOver(*score);
            }
        }

        Game { playing: playing @ false, .. } => {
            if event == Key("KeyP".into()) {
                *playing = true;
            }
        }

        GameOver(_) => {
            if event == Key("Space".into()) {
                *state = State::new_game();
            }
        }
    }
}

fn main() {
    stdweb::initialize();

    let canvas: CanvasElement = document().create_element("canvas").unwrap().try_into().unwrap();
    canvas.set_width(COLS * CELL_SIZE); canvas.set_height((ROWS - HIDDEN) * CELL_SIZE);
    canvas.set_attribute("style", CANVAS_STYLE).unwrap();
    document().body().unwrap().append_child(&canvas);

    document().body().unwrap().set_attribute("style", "background-color: #000;").unwrap();

    let mut ctx: CanvasRenderingContext2d = canvas.get_context().unwrap();

    let mut state = State::new_game();

    let (s1,r) = channel();
    let s2 = s1.clone();

    window().add_event_listener(move |e: KeyUpEvent| {
        s1.send(Key(e.code())).expect("Could not send keypress event");
    });

    js! { setInterval(@{move || {
        s2.send(Tick).expect("Could not send tick event");
    } }, 500); }

    js! {
        var callback = @{move || {
            if let Ok(event) = r.try_recv() {
                update(&mut state, event);
                render(&state, &mut ctx);
            };
        }};

        function loop() {
            callback();
            requestAnimationFrame(loop);
        }

        requestAnimationFrame(loop);
    };
}
