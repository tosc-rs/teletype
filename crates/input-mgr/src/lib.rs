#![cfg_attr(not(test), no_std)]

use core::{marker::PhantomData, ptr::NonNull};
use core::cmp::Ordering;

#[derive(Debug)]
pub struct RingLine<const L: usize, const C: usize> {
    lines: [Line<C>; L],
    brick: Bricks<L>,
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
pub struct Bricks<const L: usize> {
    idx_buf: [usize; L],
    user_editable_end: usize, //  0..ue
    inco_editable_end: usize, // ue..ie
    history_end: usize,       // ie..hi
                              // hi..   => free
}

pub struct BrickIter<'a, const L: usize, I> {
    bricks: &'a [usize],
    collection: &'a [I],
}

pub struct BrickIterMut<'a, 'b, const L: usize, I> {
    bricks: &'a [usize],
    col_ptr: NonNull<[I]>,
    _cpd: PhantomData<&'b mut [I]>,
}

impl<'a, const L: usize, I> Iterator for BrickIter<'a, L, I> {
    type Item = &'a I;

    fn next(&mut self) -> Option<Self::Item> {
        let (now, remain) = self.bricks.split_first()?;
        self.bricks = remain;
        self.collection.get(*now)
    }
}

impl<'a, 'b, const L: usize, I> Iterator for BrickIterMut<'a, 'b, L, I> {
    type Item = &'b mut I;

    fn next(&mut self) -> Option<Self::Item> {
        let (now, remain) = self.bricks.split_first()?;
        self.bricks = remain;
        unsafe { Some(&mut *self.col_ptr.as_ptr().cast::<I>().add(*now)) }
    }
}

// lower: newest
// higher: oldest

impl<const L: usize> Bricks<L> {
    pub fn new() -> Self {
        let mut idx_buf = [0; L];
        idx_buf.iter_mut().enumerate().for_each(|(i, v)| *v = i);
        Self {
            idx_buf: idx_buf,
            user_editable_end: 0,
            inco_editable_end: 0,
            history_end: 0,
        }
    }

    pub fn iter_user_editable<'a, I>(&'a self, t: &'a [I]) -> BrickIter<'a, L, I> {
        BrickIter {
            bricks: &self.idx_buf[0..self.user_editable_end],
            collection: t,
        }
    }

    pub fn iter_inco_editable<'a, I>(&'a self, t: &'a [I]) -> BrickIter<'a, L, I> {
        BrickIter {
            bricks: &self.idx_buf[self.user_editable_end..self.inco_editable_end],
            collection: t,
        }
    }

    pub fn iter_user_editable_mut<'a, 'b, I>(
        &'a self,
        t: &'b mut [I],
    ) -> BrickIterMut<'a, 'b, L, I> {
        BrickIterMut {
            bricks: &self.idx_buf[0..self.user_editable_end],
            col_ptr: NonNull::from(t),
            _cpd: PhantomData,
        }
    }

    pub fn iter_inco_editable_mut<'a, 'b, I>(
        &'a self,
        t: &'b mut [I],
    ) -> BrickIterMut<'a, 'b, L, I> {
        BrickIterMut {
            bricks: &self.idx_buf[self.user_editable_end..self.inco_editable_end],
            col_ptr: NonNull::from(t),
            _cpd: PhantomData,
        }
    }

    /// Iterate through the historical items, from NEWEST to OLDEST
    pub fn iter_history<'a, I>(&'a self, t: &'a [I]) -> BrickIter<'a, L, I> {
        BrickIter {
            bricks: &self.idx_buf[self.inco_editable_end..self.history_end],
            collection: t,
        }
    }

    pub fn pop_ue_front(&mut self) {
        if self.user_editable_end == 0 {
            return;
        }
        let end = self.history_end.wrapping_add(1).min(L);
        rot_left(&mut self.idx_buf[..end]);
        self.user_editable_end -= 1;
        self.inco_editable_end -= 1;
        self.history_end -= 1;
    }

    pub fn ue_front(&self) -> Option<usize> {
        if self.user_editable_end == 0 {
            None
        } else {
            Some(self.idx_buf[0])
        }
    }

    pub fn ie_front(&self) -> Option<usize> {
        if self.inco_editable_end == self.user_editable_end {
            None
        } else {
            Some(self.idx_buf[self.user_editable_end])
        }
    }

    // Operations:
    //
    // * Insert user editable -> Fails if all items already UE
    // * Insert inco editable -> Fails if all items already UE + IE
    // * Insert history       -> Fails if all items already UE + IE (not + history!)
    pub fn insert_ue_front(&mut self) -> Result<usize, ()> {
        if self.user_editable_end == L {
            return Err(());
        }
        // Rotate in at least one free/history
        let end = self.history_end.wrapping_add(1).min(L);
        rot_right(&mut self.idx_buf[..end]);
        self.user_editable_end = self.user_editable_end.wrapping_add(1).min(L);
        self.inco_editable_end = self.inco_editable_end.wrapping_add(1).min(L);
        self.history_end = self.history_end.wrapping_add(1).min(L);
        Ok(self.idx_buf[0])
    }

    pub fn insert_ie_front(&mut self) -> Result<usize, ()> {
        if self.inco_editable_end == L {
            return Err(());
        }
        // Rotate in at least one free/history
        let end = self.history_end.wrapping_add(1).min(L);
        rot_right(&mut self.idx_buf[self.user_editable_end..end]);
        self.inco_editable_end = self.inco_editable_end.wrapping_add(1).min(L);
        self.history_end = self.history_end.wrapping_add(1).min(L);
        Ok(self.idx_buf[self.user_editable_end])
    }

    pub fn release_ue(&mut self) {
        // We want to swap ue and ie regions.
        let range = &mut self.idx_buf[..self.inco_editable_end];

        // TODO(AJM): This is memory-friendly (requires only 1xusize extra),
        // but VERY CPU-unfriendly O(n^2) copies. This can be mitigated by
        // keeping the number of inco_editable items low, ideally 0/1.
        //
        // Alternatively, I could use O(n) extra storage, and assemble
        // the output directly.
        for _ in 0..(self.inco_editable_end - self.user_editable_end) {
            rot_right(range);
        }
        self.inco_editable_end -= self.user_editable_end;
        self.user_editable_end = 0;
    }

    pub fn release_ie(&mut self) {
        self.inco_editable_end = self.user_editable_end;
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

#[derive(Debug)]
pub struct Line<const C: usize> {
    fill: u8,
    buf: [u8; C],
    pub status: Source,
}

impl<const C: usize> Line<C> {
    pub const fn new() -> Self {
        Self {
            fill: 0,
            buf: [0u8; C],
            status: Source::Local,
        }
    }

    pub fn clear(&mut self) {
        self.fill = 0;
        self.status = Source::Local;
    }

    pub fn len(&self) -> usize {
        self.fill.into()
    }

    pub fn is_empty(&self) -> bool {
        self.fill == 0
    }

    pub fn is_full(&self) -> bool {
        self.len() >= C
    }

    pub fn pop(&mut self) {
        if self.fill != 0 {
            self.fill -= 1;
        }
    }

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

    pub fn not_full(&self) -> Result<(), LineError> {
        if self.is_full() {
            Err(LineError::Full)
        } else {
            Ok(())
        }
    }

    pub fn push(&mut self, ins: u8) -> Result<(), LineError> {
        self.not_full()?;
        ascii_good(ins)?;
        self.buf[self.len()] = ins;
        self.fill += 1;
        Ok(())
    }

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

fn ascii_good(c: u8) -> Result<(), LineError> {
    if acceptable_ascii(c) {
        Ok(())
    } else {
        Err(LineError::InvalidChar)
    }
}

fn acceptable_ascii(c: u8) -> bool {
    c.is_ascii() && !c.is_ascii_control()
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

/*

Reference coordinates:

oldest
^
|
|
|
|
0---------->rightmost

*/


#[cfg(test)]
mod test1 {
    use crate::{LineError, Source};

    use super::{Line, RingLine};

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


#[cfg(test)]
pub mod test2 {
    use super::Bricks;

    #[test]
    fn smoke() {
        let mut brick = Bricks::<8>::new();
        println!("{:?}", brick);
        for i in 0..8 {
            let x = brick.insert_ue_front().unwrap();
            println!("{:?}", brick);
            assert_eq!(x, i);
        }
        println!("{:?}", brick);
        brick.insert_ue_front().unwrap_err();
        assert_eq!(
            brick,
            Bricks {
                idx_buf: [7, 6, 5, 4, 3, 2, 1, 0],
                user_editable_end: 8,
                inco_editable_end: 8,
                history_end: 8,
            }
        );
        println!("=====");
        let mut brick = Bricks::<8>::new();
        for i in 0..4 {
            let x = brick.insert_ue_front().unwrap();
            println!("{:?}", brick);
            assert_eq!(x, i);
        }
        assert_eq!(
            brick,
            Bricks {
                idx_buf: [3, 2, 1, 0, 4, 5, 6, 7],
                user_editable_end: 4,
                inco_editable_end: 4,
                history_end: 4,
            }
        );
        println!("-----");
        for i in 4..8 {
            let x = brick.insert_ie_front().unwrap();
            println!("{:?}", brick);
            assert_eq!(x, i);
        }
        assert_eq!(
            brick,
            Bricks {
                idx_buf: [3, 2, 1, 0, 7, 6, 5, 4],
                user_editable_end: 4,
                inco_editable_end: 8,
                history_end: 8,
            }
        );
        println!("{:?}", brick);
        println!("=====");
        let mut brick = Bricks::<8>::new();
        for i in 0..3 {
            let x = brick.insert_ue_front().unwrap();
            println!("{:?}", brick);
            assert_eq!(x, i);
        }
        for i in 3..5 {
            let x = brick.insert_ie_front().unwrap();
            println!("{:?}", brick);
            assert_eq!(x, i);
        }
        println!("{:?}", brick);
        assert_eq!(
            brick,
            Bricks {
                idx_buf: [2, 1, 0, 4, 3, 5, 6, 7],
                user_editable_end: 3,
                inco_editable_end: 5,
                history_end: 5,
            }
        );
        println!("-----");
        brick.release_ue();
        println!("{:?}", brick);
        assert_eq!(
            brick,
            Bricks {
                idx_buf: [4, 3, 2, 1, 0, 5, 6, 7],
                user_editable_end: 0,
                inco_editable_end: 2,
                history_end: 5,
            }
        );
        println!("-----");
        brick.release_ie();
        println!("{:?}", brick);
        assert_eq!(
            brick,
            Bricks {
                idx_buf: [4, 3, 2, 1, 0, 5, 6, 7],
                user_editable_end: 0,
                inco_editable_end: 0,
                history_end: 5,
            }
        );
        println!("=====");
        for i in 5..8 {
            let x = brick.insert_ue_front().unwrap();
            println!("{:?}", brick);
            assert_eq!(x, i);
        }
        println!("{:?}", brick);
        assert_eq!(
            brick,
            Bricks {
                idx_buf: [7, 6, 5, 4, 3, 2, 1, 0],
                user_editable_end: 3,
                inco_editable_end: 3,
                history_end: 8,
            }
        );
        println!("-----");
        for i in 0..2 {
            let x = brick.insert_ie_front().unwrap();
            println!("{:?}", brick);
            assert_eq!(x, i);
        }
        println!("{:?}", brick);
        assert_eq!(
            brick,
            Bricks {
                idx_buf: [7, 6, 5, 1, 0, 4, 3, 2],
                user_editable_end: 3,
                inco_editable_end: 5,
                history_end: 8,
            }
        );

        let mut buf = [10, 20, 30, 40, 50, 60, 70, 80];
        assert_eq!(
            brick
                .iter_user_editable(&buf)
                .copied()
                .collect::<Vec<_>>()
                .as_slice(),
            &[80, 70, 60],
        );
        assert_eq!(
            brick
                .iter_user_editable_mut(&mut buf)
                .map(|c| *c)
                .collect::<Vec<_>>()
                .as_slice(),
            &[80, 70, 60],
        );
        assert_eq!(
            brick
                .iter_inco_editable(&buf)
                .copied()
                .collect::<Vec<_>>()
                .as_slice(),
            &[20, 10],
        );
        assert_eq!(
            brick
                .iter_inco_editable_mut(&mut buf)
                .map(|c| *c)
                .collect::<Vec<_>>()
                .as_slice(),
            &[20, 10],
        );
        assert_eq!(
            brick
                .iter_history(&buf)
                .copied()
                .collect::<Vec<_>>()
                .as_slice(),
            &[50, 40, 30],
        );

        println!("-----");
        for i in 2..5 {
            let x = brick.insert_ue_front().unwrap();
            println!("{:?}", brick);
            assert_eq!(x, i);
        }
        println!("{:?}", brick);
        assert_eq!(
            brick,
            Bricks {
                idx_buf: [4, 3, 2, 7, 6, 5, 1, 0],
                user_editable_end: 6,
                inco_editable_end: 8,
                history_end: 8,
            }
        );
        brick.insert_ie_front().unwrap_err();
        assert_eq!(brick.insert_ue_front().unwrap(), 0);
    }
}
