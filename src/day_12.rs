use core::simd;
use std::{
    ops::AddAssign,
    simd::{num::SimdInt, Simd},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use rand::{rngs::StdRng, Rng, SeedableRng};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i8)]
enum Tile {
    Empty = 0,
    Cookie = -1,
    Milk = 1,
}

// row-major board
struct Board([Tile; 16]);

const WHITE_SQUARE: &str = "⬜";
const COOKIE_EMOJI: &str = "🍪";
const BLACK_SQUARE: &str = "⬛";
const MILK_GLASS: &str = "🥛";

impl Board {
    pub fn decode(state: u64) -> Self {
        let mut tiles = [Tile::Empty; 16];
        for i in 0..16 {
            let val = ((state >> (2 * i)) & 0b11) as u8;
            tiles[i] = match val {
                0 => Tile::Empty,
                1 => Tile::Cookie,
                2 => Tile::Milk,
                _ => unreachable!(),
            };
        }
        Board(tiles)
    }

    pub fn encode(&self) -> u64 {
        let mut state = 0_u64;
        for (i, &tile) in self.0.iter().enumerate() {
            let tile = match tile {
                Tile::Empty => 0,
                Tile::Cookie => 1,
                Tile::Milk => 2,
            };
            state |= (tile as u64) << (2 * i);
        }
        state
    }

    pub fn new_random(rng: &mut rand::rngs::StdRng) -> Self {
        let mut board = [Tile::Empty; 16];
        for item in board.iter_mut() {
            *item = match rng.gen::<bool>() {
                true => Tile::Cookie,
                false => Tile::Milk,
            }
        }
        Self(board)
    }

    pub fn render(&self) -> String {
        let mut s = String::new();

        for row in 0..4 {
            s.push_str(WHITE_SQUARE);
            for col in 0..4 {
                let tile = self.0[row * 4 + col];
                let ch = match tile {
                    Tile::Empty => BLACK_SQUARE,
                    Tile::Cookie => COOKIE_EMOJI,
                    Tile::Milk => MILK_GLASS,
                };
                s.push_str(ch);
            }
            s.push_str(WHITE_SQUARE);
            s.push('\n');
        }

        s.push_str(&WHITE_SQUARE.repeat(6));

        match self.check_for_winner() {
            Ok(Some(winner)) => {
                s.push('\n');
                s.push_str(match winner {
                    Tile::Cookie => "🍪 wins!",
                    Tile::Milk => "🥛 wins!",
                    _ => unreachable!(),
                });
            }
            Err(_) => {
                s.push('\n');
                s.push_str("No winner.");
            }
            _ => {}
        }
        s.push('\n');

        s
    }

    fn check_for_winner(&self) -> Result<Option<Tile>, ()> {
        fn check_value(val: i8) -> Option<Tile> {
            match val {
                4 => Some(Tile::Milk),
                -4 => Some(Tile::Cookie),
                _ => None,
            }
        }

        // We'll keep track of row sums and column sums using SIMD vectors.
        // Initialize everything to zero.
        let mut winner_cols = Simd::from_array([0i8; 4]);
        let mut winner_rows = [0i8; 4];

        let mut winner_d_top_to_bot = 0i8;
        let mut winner_d_bot_to_top = 0i8;

        // Iterate through rows and zip them with the winner_rows iterator
        for ((row, line), winner_row) in (0..4)
            .zip(self.0.chunks_exact(4))
            .zip(winner_rows.iter_mut())
        {
            let row_line =
                Simd::from_array([line[0] as i8, line[1] as i8, line[2] as i8, line[3] as i8]);

            // Update column sums using SIMD addition
            winner_cols += row_line;

            // Update the corresponding row sum using a SIMD reduction
            *winner_row += row_line.reduce_sum();

            // Update diagonals
            winner_d_top_to_bot.add_assign(line[row] as i8);
            winner_d_bot_to_top.add_assign(line[3 - row] as i8);
        }
        let winner_cols_arr = winner_cols.to_array();

        // Combine all values into a single iterator
        let diagonal_check = [winner_d_top_to_bot, winner_d_bot_to_top];
        let all_results = winner_cols_arr
            .iter()
            .chain(winner_rows.iter())
            .chain(diagonal_check.iter());

        // Check each value
        for &val in all_results {
            if let Some(tile) = check_value(val) {
                return Ok(Some(tile));
            }
        }

        // Check for draw: if no empty slots are left, it's a tie
        if !self.0.contains(&Tile::Empty) {
            Err(())
        } else {
            Ok(None)
        }
    }

    fn get_col(&self, col: usize) -> [Tile; 4] {
        let line = [
            self.0[col],
            self.0[col + 4],
            self.0[col + 8],
            self.0[col + 12],
        ];
        line
    }

    fn push_item(&mut self, col_idx: usize, item: Tile) -> Result<(), ()> {
        let col = self.get_col(col_idx);
        let (idx_last_empty, _) = col
            .iter()
            .enumerate()
            .rev()
            .find(|(_idx, x)| **x == Tile::Empty)
            .ok_or(())?;
        self.0[col_idx + (idx_last_empty * 4)] = item;
        Ok(())
    }
}

static BOARD: AtomicU64 = AtomicU64::new(0);

pub async fn board() -> Response {
    let s = render_board();

    (StatusCode::OK, s).into_response()
}

fn render_board() -> String {
    let board = BOARD.load(Ordering::Relaxed);
    let board = Board::decode(board);
    let s = board.render();
    s
}

pub async fn reset(State(rng): State<Arc<Mutex<StdRng>>>) -> Response {
    let mut rng = rng.lock().unwrap();
    *rng = rand::rngs::StdRng::seed_from_u64(2024);
    drop(rng);

    let _board = BOARD
        .fetch_update(Ordering::Release, Ordering::Acquire, |_old_state| Some(0))
        .unwrap();

    let s = render_board();

    (StatusCode::OK, s).into_response()
}

pub async fn random_board(State(rng): State<Arc<Mutex<StdRng>>>) -> Response {
    let mut rng = rng.lock().unwrap();
    let board = Board::new_random(&mut rng);
    drop(rng);
    BOARD.store(board.encode(), Ordering::Relaxed);

    let s = render_board();

    (StatusCode::OK, s).into_response()
}

pub async fn place(Path((team, column)): Path<(String, String)>) -> Response {
    let team = match team.as_str() {
        "cookie" => Tile::Cookie,
        "milk" => Tile::Milk,
        _ => return (StatusCode::BAD_REQUEST,).into_response(),
    };

    let column = match column.parse::<usize>() {
        Ok(c) if (1..=4).contains(&c) => c - 1,
        _ => return (StatusCode::BAD_REQUEST,).into_response(),
    };

    let board_val = BOARD.load(Ordering::Relaxed);
    let board = Board::decode(board_val);

    // Early check if game over
    let state = board.check_for_winner();
    if state.is_err() || state.ok().flatten().is_some() {
        return (StatusCode::SERVICE_UNAVAILABLE, render_board()).into_response();
    }

    let res = BOARD.fetch_update(Ordering::Release, Ordering::Acquire, |old| {
        let mut b = Board::decode(old);
        b.push_item(column, team).ok().map(|_| b.encode())
    });

    let s = render_board();
    match res {
        Ok(new_val) => {
            let new_board = Board::decode(new_val);
            match new_board.check_for_winner() {
                Ok(Some(_)) => (StatusCode::OK, s).into_response(),
                Ok(None) => (StatusCode::OK, s).into_response(),
                Err(_) => (StatusCode::SERVICE_UNAVAILABLE, s).into_response(),
            }
        }
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE,).into_response(),
    }
}
