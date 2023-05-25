#![cfg_attr(not(test), no_std)]

mod lines;
mod bricks;

pub use lines::Line;
pub use bricks::{BrickIter, BrickIterMut};
use bricks::Bricks;

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
    pub fn iter_history(&self) -> BrickIter<'_, L, Line<C>> {
        let Self { lines, brick } = self;
        brick.iter_history(lines)
    }

    /// Iterates any lines that are currently being edited by the remote end, NEWEST to OLDEST
    pub fn iter_remote_editing(&self) -> BrickIter<'_, L, Line<C>> {
        let Self { lines, brick } = self;
        brick.iter_inco_editable(lines)
    }

    /// Iterates any lines that are currently being edited by the local end, NEWEST to OLDEST
    pub fn iter_local_editing(&self) -> BrickIter<'_, L, Line<C>> {
        let Self { lines, brick } = self;
        brick.iter_user_editable(lines)
    }

    /// Iterates any lines that are currently being edited by the remote end, NEWEST to OLDEST
    pub fn iter_remote_editing_mut(&mut self) -> BrickIterMut<'_, '_, L, Line<C>> {
        let Self { lines, brick } = self;
        brick.iter_inco_editable_mut(lines)
    }

    /// Iterates any lines that are currently being edited by the local end, NEWEST to OLDEST
    pub fn iter_local_editing_mut(&mut self) -> BrickIterMut<'_, '_, L, Line<C>> {
        let Self { lines, brick } = self;
        brick.iter_user_editable_mut(lines)
    }

    /// Moves the local editing region into a user historical region
    pub fn submit_local_editing(&mut self) {
        self.brick.release_ue();
    }

    /// Moves the remote editing region into a user historical region
    pub fn submit_remote_editing(&mut self) {
        self.brick.release_ie();
    }

    /// Attempts to append a character to the local editing region
    pub fn append_local_char(&mut self, c: u8) -> Result<(), RingLineError> {
        self.get_local_first_writeable()
            .ok_or(RingLineError::Line(LineError::Full))?
            .push(c)?;
        Ok(())
    }

    /// Attempts to append a character to the remote editing region
    pub fn append_remote_char(&mut self, c: u8) -> Result<(), RingLineError> {
        self.get_remote_first_writeable()
            .ok_or(RingLineError::Line(LineError::Full))?
            .push(c)?;
        Ok(())
    }

    /// Attempts to remove a character from the local editing region
    pub fn pop_local_char(&mut self) {
        let Self { lines, brick } = self;
        if let Some(cur) = brick.iter_user_editable_mut(lines).next() {
            if cur.is_empty() {
                brick.pop_ue_front();
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
        let wr = if let Some(wr) = brick.ue_front() {
            let cur = &lines[wr];
            if cur.is_full() {
                new = true;
                self.brick.insert_ue_front().ok()?
            } else {
                wr
            }
        } else {
            new = true;
            self.brick.insert_ue_front().ok()?
        };
        let cur = &mut lines[wr];
        if new {
            cur.clear();
            cur.status = Source::Local;
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
        let wr = if let Some(wr) = brick.ie_front() {
            let cur = &lines[wr];
            if cur.is_full() {
                new = true;
                self.brick.insert_ie_front().ok()?
            } else {
                wr
            }
        } else {
            new = true;
            self.brick.insert_ie_front().ok()?
        };
        let cur = &mut lines[wr];
        if new {
            cur.clear();
            cur.status = Source::Remote;
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
