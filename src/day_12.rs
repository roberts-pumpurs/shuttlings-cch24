use std::sync::atomic::{AtomicU64, Ordering};

use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
enum Tile {
    Empty,
    Cookie,
    Milk,
}

// row-major board
struct Board([Tile; 16]);

const WHITE_SQUARE: &str = "⬜";
const COOKIE_EMOJI: &str = "🍪";
const BLACK_SQUARE: &str = "⬛";
const MILK_GLASS: &str = "🥛";

const MAX_RENDERED_BOARD_SIZE: usize = 128;

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
            state |= (tile as u64) << (2 * i);
        }
        state
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
        // Helper function to check if four tiles form a winning line
        let is_winning_line =
            |line: &[Tile]| line[0] != Tile::Empty && line.iter().all(|&t| t == line[0]);

        // Check rows
        for row in 0..4 {
            let start = row * 4;
            let line = &self.0[start..start + 4];
            if is_winning_line(line) {
                return Ok(Some(line[0]));
            }
        }

        // Check columns
        for col in 0..4 {
            let line = self.get_col(col);
            if is_winning_line(&line) {
                return Ok(Some(line[0]));
            }
        }

        // Check diagonals
        let diag1 = [self.0[0], self.0[5], self.0[10], self.0[15]];
        if is_winning_line(&diag1) {
            return Ok(Some(diag1[0]));
        }

        let diag2 = [self.0[3], self.0[6], self.0[9], self.0[12]];
        if is_winning_line(&diag2) {
            return Ok(Some(diag2[0]));
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

pub async fn reset() -> Response {
    let _board = BOARD
        .fetch_update(Ordering::Release, Ordering::Acquire, |_old_state| Some(0))
        .unwrap();

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
