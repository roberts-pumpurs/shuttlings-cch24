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

    pub fn render(&self) -> (usize, [u8; MAX_RENDERED_BOARD_SIZE]) {
        let mut buf = [0_u8; MAX_RENDERED_BOARD_SIZE];
        let mut offset = 0;

        // Helper to write a string into our buffer
        let mut write_str = |s: &str| {
            let bytes = s.as_bytes();
            buf[offset..offset + bytes.len()].copy_from_slice(bytes);
            offset += bytes.len();
        };

        // We print 5 rows total:
        // For the first 4 rows, we print:
        //   White square, then each of the 4 tiles, then white square
        // For the last row, we print 6 white squares.

        // Each of the first 4 rows
        for row in 0..4 {
            write_str(WHITE_SQUARE);
            for col in 0..4 {
                let tile = self.0[row * 4 + col];
                match tile {
                    Tile::Empty => write_str(BLACK_SQUARE),
                    Tile::Cookie => write_str(COOKIE_EMOJI),
                    Tile::Milk => write_str(MILK_GLASS),
                }
            }
            write_str(WHITE_SQUARE);
            // newline
            write_str("\n");
        }

        // Last row: 6 white squares
        for _ in 0..6 {
            write_str(WHITE_SQUARE);
        }

        match self.check_for_winner() {
            Ok(Some(winner)) => {
                write_str("\n");
                let suffix = match winner {
                    Tile::Empty => unreachable!(),
                    Tile::Cookie => "🍪 wins!",
                    Tile::Milk => "🥛 wins!",
                };
                write_str(suffix);
            }
            Err(_) => {
                write_str("\n");
                write_str("No winner.");
            }
            _ => {}
        }
        write_str("\n");

        (offset, buf)
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
    let (len, buf) = board.render();
    // todo this can be optimised by movig to a shared buffer for all instances
    let s = std::str::from_utf8(&buf[..len]).unwrap().to_string();
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

    let mut column: usize = match column.parse() {
        Ok(res) => res,
        Err(_) => return (StatusCode::BAD_REQUEST,).into_response(),
    };
    if column < 1 {
        return (StatusCode::BAD_REQUEST,).into_response();
    }
    column = column.saturating_sub(1);
    if column >= 4 {
        return (StatusCode::BAD_REQUEST,).into_response();
    }
    let board = BOARD.load(Ordering::Relaxed);
    let board = Board::decode(board);
    if board.check_for_winner().is_err()
        || board
            .check_for_winner()
            .map(|x| x.is_some())
            .unwrap_or(false)
    {
        let s = render_board();
        return (StatusCode::SERVICE_UNAVAILABLE, s).into_response();
    }

    let board = BOARD.fetch_update(Ordering::Release, Ordering::Acquire, |board| {
        let mut board = Board::decode(board);

        board.push_item(column, team).map(|_| board.encode()).ok()
    });

    let s = render_board();

    tracing::info!("{s:}");
    match board {
        Ok(board) => {
            let board = Board::decode(board);
            match board.check_for_winner() {
                // has a winner
                Ok(Some(_winner)) => return (StatusCode::OK, s).into_response(),
                // no winner yet
                Ok(None) => return (StatusCode::OK, s).into_response(),
                // the board cannot have a winner, board is full
                Err(_) => return (StatusCode::SERVICE_UNAVAILABLE, s).into_response(),
            }
        }
        // the column is full
        Err(_) => return (StatusCode::SERVICE_UNAVAILABLE,).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_empty_board() {
        let board = Board([Tile::Empty; 16]);
        let (len, buf) = board.render();
        let output = std::str::from_utf8(&buf[..len]).unwrap();

        let expected = concat!(
            "⬜⬛⬛⬛⬛⬜\n",
            "⬜⬛⬛⬛⬛⬜\n",
            "⬜⬛⬛⬛⬛⬜\n",
            "⬜⬛⬛⬛⬛⬜\n",
            "⬜⬜⬜⬜⬜⬜\n"
        );

        assert_eq!(
            output, expected,
            "Empty board does not match expected output"
        );
    }

    #[test]
    fn test_render_with_cookies() {
        let tiles = [
            Tile::Cookie,
            Tile::Empty,
            Tile::Empty,
            Tile::Empty, // Row 1
            Tile::Cookie,
            Tile::Empty,
            Tile::Empty,
            Tile::Empty, // Row 2
            Tile::Cookie,
            Tile::Empty,
            Tile::Empty,
            Tile::Empty, // Row 3
            Tile::Cookie,
            Tile::Empty,
            Tile::Empty,
            Tile::Empty, // Row 4
        ];
        let board = Board(tiles);
        let (len, buf) = board.render();
        let output = std::str::from_utf8(&buf[..len]).unwrap();

        let expected = concat!(
            "⬜🍪⬛⬛⬛⬜\n",
            "⬜🍪⬛⬛⬛⬜\n",
            "⬜🍪⬛⬛⬛⬜\n",
            "⬜🍪⬛⬛⬛⬜\n",
            "⬜⬜⬜⬜⬜⬜\n"
        );

        assert_eq!(
            output, expected,
            "Cookie board does not match expected output"
        );
    }

    #[test]
    fn test_round_trip_encode_decode() {
        // Create a board pattern and ensure we can encode and decode it,
        // and that render remains stable after a round-trip.

        let tiles = [
            Tile::Cookie,
            Tile::Empty,
            Tile::Empty,
            Tile::Milk,
            Tile::Empty,
            Tile::Cookie,
            Tile::Milk,
            Tile::Empty,
            Tile::Milk,
            Tile::Milk,
            Tile::Cookie,
            Tile::Empty,
            Tile::Empty,
            Tile::Empty,
            Tile::Empty,
            Tile::Cookie,
        ];

        let original = Board(tiles);
        let encoded = original.encode();
        let decoded = Board::decode(encoded);

        assert_eq!(
            original.0, decoded.0,
            "Tiles differ after encode/decode round-trip"
        );

        let (original_len, original_buf) = original.render();
        let (decoded_len, decoded_buf) = decoded.render();

        let original_str = std::str::from_utf8(&original_buf[..original_len]).unwrap();
        let decoded_str = std::str::from_utf8(&decoded_buf[..decoded_len]).unwrap();

        assert_eq!(
            original_str, decoded_str,
            "Render output differs after encode/decode round-trip"
        );
    }
}
