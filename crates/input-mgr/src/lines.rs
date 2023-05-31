//! # Lines
//!
//! Lines generally act as a vector of ascii characters, with a fixed upper size.
//!
//! At the moment, lines must be <= 255 bytes long.

use core::cmp::Ordering;

use crate::{rot_right, LineError, Source};

/// A single line with fixed capacity for C characters
///
/// At the moment, `C` must be <= 255.
///
/// This [Line] acts more or less like a small, fixed size vector of ascii characters
///
/// Lines ONLY contain [valid ascii] characters, that are not [control characters].
///
/// [valid ascii]: u8::is_ascii()
/// [control characters]: u8::is_ascii_control()
#[derive(Debug)]
pub struct Line<const C: usize> {
    fill: u8,
    buf: [u8; C],
    status: Source,
}

impl<const C: usize> Line<C> {
    /// Create a new empty line
    pub const fn new() -> Self {
        Self {
            fill: 0,
            buf: [0u8; C],
            status: Source::Local,
        }
    }

    /// The source of the current line, either Local or Remote
    pub fn status(&self) -> Source {
        self.status
    }

    /// Set the status (internal interface)
    pub(crate) fn set_status(&mut self, source: Source) {
        self.status = source;
    }

    /// Clear the line (internal interface)
    ///
    /// NOTE: this is private because this line should only be cleared like this
    /// when also updating the [Bricks] tracking information
    pub(crate) fn clear(&mut self) {
        self.fill = 0;
        self.status = Source::Local;
    }

    /// The currently used number of bytes in this line
    pub fn len(&self) -> usize {
        self.fill.into()
    }

    /// Is the current line empty?
    pub fn is_empty(&self) -> bool {
        self.fill == 0
    }

    /// Is the current line full?
    pub fn is_full(&self) -> bool {
        self.len() >= C
    }

    /// Pop a character from the END of the line (if any)
    pub fn pop(&mut self) {
        if self.fill != 0 {
            self.fill -= 1;
        }
    }

    /// Obtain the current line as a `&str`
    pub fn as_str(&self) -> &str {
        self.buf
            .get(..self.len())
            .and_then(|s| core::str::from_utf8(s).ok())
            .unwrap_or("")
    }

    pub const fn cap_u8() -> u8 {
        if C > ((u8::MAX - 1) as usize) {
            panic!("Too big!")
        } else {
            C as u8
        }
    }

    /// Extend the current line with the given string slice.
    ///
    /// Returns an error if the provided slice would not fit, or if
    /// any characters are invalid ascii or ascii control characters
    pub fn extend(&mut self, s: &str) -> Result<(), LineError> {
        let len = self.len();

        if len + s.len() > C {
            return Err(LineError::Full);
        }
        if !s.as_bytes().iter().copied().all(acceptable_ascii) {
            return Err(LineError::InvalidChar);
        }
        self.buf[len..][..s.len()].copy_from_slice(s.as_bytes());
        self.fill += s.len() as u8;
        Ok(())
    }

    /// Overwrite an ascii character at the given position
    ///
    /// Returns an error if the provided location would leave a gap (beyond the
    /// end of the current line), or if the given character is invalid ascii or
    /// an ascii control character
    pub fn overwrite(&mut self, pos: usize, ovrw: u8) -> Result<(), LineError> {
        if pos > self.len() || pos >= C {
            return Err(LineError::Full);
        }
        ascii_good(ovrw)?;

        self.buf[pos] = ovrw;
        if pos == self.len() {
            self.fill += 1;
        }
        Ok(())
    }

    /// Returns an error if the line is full
    pub fn not_full(&self) -> Result<(), LineError> {
        if self.is_full() {
            Err(LineError::Full)
        } else {
            Ok(())
        }
    }

    /// Push an ascii character to the end of the line
    ///
    /// Returns an error if the provided character would not fit, or if
    /// the character is invalid ascii or an ascii control character
    pub fn push(&mut self, ins: u8) -> Result<(), LineError> {
        self.not_full()?;
        ascii_good(ins)?;
        self.buf[self.len()] = ins;
        self.fill += 1;
        Ok(())
    }

    /// Insert an ascii character at the given position
    ///
    /// Returns an error if the provided location would leave a gap (beyond the
    /// end of the current line), or if the given character is invalid ascii or
    /// an ascii control character, or if the line is already full
    pub fn insert(&mut self, pos: usize, ins: u8) -> Result<(), LineError> {
        self.not_full()?;

        if pos >= C {
            return Err(LineError::Full);
        }
        if !acceptable_ascii(ins) {
            return Err(LineError::InvalidChar);
        }

        match self.len().cmp(&pos) {
            Ordering::Equal => {
                self.buf[pos] = ins;
                self.fill += 1;
                Ok(())
            }
            Ordering::Greater => {
                let len = self.len();
                self.buf[len] = ins;
                rot_right(&mut self.buf[..len + 1]);
                self.fill += 1;
                Ok(())
            }
            Ordering::Less => return Err(LineError::WriteGap), // trying to insert AFTER the "tip"
        }
    }
}

fn acceptable_ascii(c: u8) -> bool {
    c.is_ascii() && !c.is_ascii_control()
}

fn ascii_good(c: u8) -> Result<(), LineError> {
    if acceptable_ascii(c) {
        Ok(())
    } else {
        Err(LineError::InvalidChar)
    }
}

#[cfg(test)]
mod line_tests {
    use crate::LineError;

    use super::Line;

    #[test]
    fn ascii() {
        assert!(b'\r'.is_ascii());
        assert!(b'\r'.is_ascii_control());
        assert!(b'\n'.is_ascii());
        assert!(b'\n'.is_ascii_control());
    }

    #[test]
    fn smoke_ring() {}

    #[test]
    fn smoke_line() {
        let mut line = Line::<10>::new();
        assert_eq!(line.as_str(), "");
        for (i, c) in b"hello".iter().enumerate() {
            line.insert(i, *c).unwrap();
            assert_eq!(line.as_str(), &"hello"[..(i + 1)]);
        }
        for i in (line.len() + 1)..256 {
            assert!(matches!(
                line.insert(i, b' ').unwrap_err(),
                LineError::WriteGap | LineError::Full
            ));
        }
        for c in b"world" {
            line.insert(0, *c).unwrap();
        }
        assert_eq!(line.as_str(), "dlrowhello");
        for i in 0..256 {
            assert_eq!(line.insert(i, b' ').unwrap_err(), LineError::Full);
        }
        for i in 0..line.len() {
            line.overwrite(i, b'a').unwrap();
        }
        assert_eq!(line.as_str(), "aaaaaaaaaa");
        for i in line.len()..256 {
            assert_eq!(line.insert(i, b' ').unwrap_err(), LineError::Full);
        }

        line.clear();
        assert_eq!(line.as_str(), "");
        for i in 1..256 {
            assert!(matches!(
                line.overwrite(i, b' ').unwrap_err(),
                LineError::WriteGap | LineError::Full
            ));
            assert!(matches!(
                line.insert(i, b' ').unwrap_err(),
                LineError::WriteGap | LineError::Full
            ));
        }
        line.overwrite(0, b'a').unwrap();
        assert_eq!(line.as_str(), "a");
        line.overwrite(0, b'b').unwrap();
        assert_eq!(line.as_str(), "b");
        line.clear();
        line.extend("hello").unwrap();
        line.extend("world").unwrap();
        assert_eq!(line.as_str(), "helloworld");

        line.pop();
        assert_eq!(line.as_str(), "helloworl");

        line.pop();
        assert_eq!(line.as_str(), "hellowor");

        line.clear();
        assert_eq!(
            line.extend("hello\nworl").unwrap_err(),
            LineError::InvalidChar
        );
        assert_eq!(
            line.extend("hello\rworl").unwrap_err(),
            LineError::InvalidChar
        );
        assert_eq!(line.extend("Späti").unwrap_err(), LineError::InvalidChar);
        assert_eq!(line.as_str(), "");
    }
}
