//! # Input Manager
//!
//! Input manager is a ring-buffer based method of representing input
//! and output of a system.
//!
//! It intends to provide a similar function to a tty/terminal, but uses
//! a "chat bubble" metaphor to distinguish from "local input" and "remote
//! output".
//!
//! In a typical tty application, "Local" would correspond to stdin, and "Remote"
//! would correspond to "stdout".
//!
//! It uses a fixed-size array of lines for storage, where each line can hold
//! a dynamic number of characters. These lines can be cheaply reordered (using
//! a separate index for the ordering of lines).
//!
//! These lines are sorted into four distinct regions:
//!
//! 1. "Local Editing Region" - or lines that the user is currently typing in,
//!    but have not submitted. This is like the text box in a chat program - it
//!    is not "latched in" until you hit the send button (or in our case, you
//!    submit the lines.
//! 2. "Remote Editing Region" - or lines that the computer is currently typing in,
//!    but has not submitted. This would be like if you could preview the message
//!    being written by someone else in a chat program, before they hit send.
//! 3. "History region" - A listing of the most recent lines that have been
//!    submitted. This is like the history of a chat window. Each line is tagged
//!    with its source, either from the local or remote end.
//! 4. Empty lines, that have never been used for local or remote input. Initially
//!    all lines are empty lines, however eventually most lines will end up being
//!    editing or history lines, and there will be no empty lines
//!
//! When the local or remote end wants an additional line for editing, the history
//! lines (or empty lines, if available) will be recycled to become

#![cfg_attr(not(test), no_std)]

mod bricks;
mod lines;

use bricks::Bricks;
pub use bricks::{LineIter, LineIterMut};
pub use lines::Line;

/// # RingLine
///
/// The [RingLine] structure contains the textual history of a two entity conversation
///
/// It is generic over two numbers:
///
/// * `L` is the number of lines it can store
/// * `C` is the maximum number of ASCII characters per line
///
/// In general, `L` should be >= the number of lines you intend to display. If L is
/// larger than the number of lines you would like to display, it can also be used
/// as a "scrollback" buffer.
///
/// RingLine does NOT store lines in a "sparse" manner - if you have 16 lines and 80
/// characters per line, 1280 bytes will be used to store those characters, even if
/// all lines are blank.
#[derive(Debug)]
pub struct RingLine<const L: usize, const C: usize> {
    lines: [Line<C>; L],
    brick: Bricks<L>,
}

impl<const L: usize, const C: usize> RingLine<L, C> {
    const ONELINE: Line<C> = Line::<C>::new();
    const INIT: [Line<C>; L] = [Self::ONELINE; L];

    pub fn new() -> Self {
        Self {
            lines: Self::INIT,
            brick: Bricks::new(),
        }
    }

    /// Iterates all "historical" (e.g. not currently editing) lines, NEWEST to OLDEST
    ///
    /// Each line contans a status field that marks it as local or remote
    pub fn iter_history(&self) -> LineIter<'_, L, Line<C>> {
        let Self { lines, brick } = self;
        brick.iter_history(lines)
    }

    /// Iterates any lines that are currently being edited by the remote end, NEWEST to OLDEST
    pub fn iter_remote_editing(&self) -> LineIter<'_, L, Line<C>> {
        let Self { lines, brick } = self;
        brick.iter_remote_editable(lines)
    }

    /// Iterates any lines that are currently being edited by the local end, NEWEST to OLDEST
    pub fn iter_local_editing(&self) -> LineIter<'_, L, Line<C>> {
        let Self { lines, brick } = self;
        brick.iter_local_editable(lines)
    }

    /// Iterates any lines that are currently being edited by the remote end, NEWEST to OLDEST
    pub fn iter_remote_editing_mut(&mut self) -> LineIterMut<'_, '_, L, Line<C>> {
        let Self { lines, brick } = self;
        brick.iter_remote_editable_mut(lines)
    }

    /// Iterates any lines that are currently being edited by the local end, NEWEST to OLDEST
    pub fn iter_local_editing_mut(&mut self) -> LineIterMut<'_, '_, L, Line<C>> {
        let Self { lines, brick } = self;
        brick.iter_local_editable_mut(lines)
    }

    /// Moves the local editing region into a user historical region
    pub fn submit_local_editing(&mut self) {
        self.brick.submit_local_editable();
    }

    /// Moves the remote editing region into a user historical region
    pub fn submit_remote_editing(&mut self) {
        self.brick.submit_remote_editable();
    }

    /// Attempts to append a character to the local editing region
    ///
    /// Does NOT accept control characters, such as `\n`.
    pub fn append_local_char(&mut self, c: u8) -> Result<(), RingLineError> {
        self.get_local_first_writeable()
            .ok_or(RingLineError::Line(LineError::Full))?
            .push(c)?;
        Ok(())
    }

    /// Attempts to append a character to the remote editing region
    ///
    /// Does NOT accept control characters, such as `\n`.
    pub fn append_remote_char(&mut self, c: u8) -> Result<(), RingLineError> {
        self.get_remote_first_writeable()
            .ok_or(RingLineError::Line(LineError::Full))?
            .push(c)?;
        Ok(())
    }

    /// Attempts to remove a character from the local editing region
    pub fn pop_local_char(&mut self) {
        let Self { lines, brick } = self;
        if let Some(cur) = brick.iter_local_editable_mut(lines).next() {
            if cur.is_empty() {
                brick.pop_local_editable_front();
            } else {
                cur.pop();
            }
        }
    }

    /// Attempts to remove a character from the local editing region
    pub fn pop_remote_char(&mut self) {
        let Self { lines, brick } = self;
        if let Some(cur) = brick.iter_remote_editable_mut(lines).next() {
            if cur.is_empty() {
                brick.pop_remote_editable_front();
            } else {
                cur.pop();
            }
        }
    }

    fn get_local_first_writeable(&mut self) -> Option<&mut Line<C>> {
        let Self { lines, brick } = self;
        // If empty, make a new one and return
        // If not empty, is the head writable and !full? => return
        // else, if not full make a new one and return
        // else, remove oldest, make a new one and return
        let mut new = false;
        let wr = if let Some(wr) = brick.local_editable_front() {
            let cur = &lines[wr];
            if cur.is_full() {
                new = true;
                self.brick.insert_local_editable_front().ok()?
            } else {
                wr
            }
        } else {
            new = true;
            self.brick.insert_local_editable_front().ok()?
        };
        let cur = &mut lines[wr];
        if new {
            cur.clear();
            cur.set_status(Source::Local);
        }

        Some(cur)
    }

    fn get_remote_first_writeable(&mut self) -> Option<&mut Line<C>> {
        let Self { lines, brick } = self;
        // If empty, make a new one and return
        // If not empty, is the head writable and !full? => return
        // else, if not full make a new one and return
        // else, remove oldest, make a new one and return
        let mut new = false;
        let wr = if let Some(wr) = brick.remote_editable_front() {
            let cur = &lines[wr];
            if cur.is_full() {
                new = true;
                self.brick.insert_remote_editable_front().ok()?
            } else {
                wr
            }
        } else {
            new = true;
            self.brick.insert_remote_editable_front().ok()?
        };
        let cur = &mut lines[wr];
        if new {
            cur.clear();
            cur.set_status(Source::Remote);
        }

        Some(cur)
    }
}

#[derive(Debug, PartialEq)]
pub enum RingLineError {
    Line(LineError),
}

impl From<LineError> for RingLineError {
    fn from(le: LineError) -> Self {
        RingLineError::Line(le)
    }
}

#[derive(Debug, PartialEq)]
pub enum LineError {
    Full,
    InvalidChar,
    ReadOnly,
    WriteGap,
}

#[derive(Debug, PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum Source {
    Local,
    Remote,
}

#[inline]
pub(crate) fn rot_right<T: Sized>(sli: &mut [T]) {
    let len = sli.len();
    if len <= 1 {
        // Look, it's rotated!
        return;
    }
    unsafe {
        let ptr = sli.as_mut_ptr();
        let last_val = ptr.add(len - 1).read();
        core::ptr::copy(ptr, ptr.add(1), len - 1);
        ptr.write(last_val);
    }
}

#[inline]
pub(crate) fn rot_left<T: Sized>(sli: &mut [T]) {
    let len = sli.len();
    if len <= 1 {
        // Look, it's rotated!
        return;
    }
    unsafe {
        let ptr = sli.as_mut_ptr();
        let first_val = ptr.read();
        core::ptr::copy(ptr.add(1), ptr, len - 1);
        ptr.add(len - 1).write(first_val);
    }
}
