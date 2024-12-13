use std::{
    cell::{Cell, LazyCell, RefCell},
    ops::Div,
    sync::{
        atomic::{AtomicU64, AtomicU8, Ordering},
        LazyLock, Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::Bytes,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

fn encode_state(bucket_size: u8, timestamp_ms: u64) -> u64 {
    let mut encoded = [0_u8; 8];
    for (idx, byte) in timestamp_ms.to_le_bytes().into_iter().enumerate().skip(1) {
        encoded[idx] = byte;
    }
    encoded[0] = bucket_size;
    u64::from_le_bytes(encoded)
}

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
        write_str("\n");

        (offset, buf)
    }
}

static BOARD: AtomicU64 = AtomicU64::new(0);

pub async fn board() -> Response {
    // calculate the amount of time between the last time we withdrew a single milk
    let board = BOARD.load(Ordering::Relaxed);
    let board = Board::decode(board);
    let (len, buf) = board.render();
    // todo this can be optimised by movig to a shared buffer for all instances
    let s = std::str::from_utf8(&buf[..len]).unwrap().to_string();

    (StatusCode::OK, s).into_response()
}

pub async fn reset() -> Response {
    let board = BOARD
        .fetch_update(Ordering::Release, Ordering::Acquire, |_old_state| Some(0))
        .unwrap();

    let board = Board::decode(board);
    let (len, buf) = board.render();
    // todo this can be optimised by movig to a shared buffer for all instances
    let s = std::str::from_utf8(&buf[..len]).unwrap().to_string();

    (StatusCode::OK, s).into_response()
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
