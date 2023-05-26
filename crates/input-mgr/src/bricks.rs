//! # Bricks
//!
//! Bricks is a vaguely named data structure that is responsible for maintaining
//! the ordering and purpose of various lines.
//!
//! It serves as one layer of indirection, to allow lines to appear to "move up"
//! in the history, without actually having to ever move the lines in memory: only
//! the order of line-indexes are ever modified.
//!
//! Lines are sorted by their purpose:
//!
//! * Local editing lines, followed by
//! * Remote editing lines, followed by
//! * History lines, followed by
//! * Empty lines
//!
//! [Bricks] is also used to provide an iterator over lines.

use core::{marker::PhantomData, ptr::NonNull};

use crate::{rot_left, rot_right};

#[derive(Debug, PartialEq)]
pub(crate) struct Bricks<const L: usize> {
    idx_buf: [usize; L],
    local_editable_end: usize,  //  0..le
    remote_editable_end: usize, // le..re
    history_end: usize,         // re..hi
                                // hi..   => free
}

pub struct LineIter<'a, const L: usize, I> {
    bricks: &'a [usize],
    collection: &'a [I],
}

pub struct LineIterMut<'a, 'b, const L: usize, I> {
    bricks: &'a [usize],
    col_ptr: NonNull<[I]>,
    _cpd: PhantomData<&'b mut [I]>,
}

impl<'a, const L: usize, I> Iterator for LineIter<'a, L, I> {
    type Item = &'a I;

    fn next(&mut self) -> Option<Self::Item> {
        let (now, remain) = self.bricks.split_first()?;
        self.bricks = remain;
        self.collection.get(*now)
    }
}

impl<'a, 'b, const L: usize, I> Iterator for LineIterMut<'a, 'b, L, I> {
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
            local_editable_end: 0,
            remote_editable_end: 0,
            history_end: 0,
        }
    }

    pub fn iter_local_editable<'a, I>(&'a self, t: &'a [I]) -> LineIter<'a, L, I> {
        LineIter {
            bricks: &self.idx_buf[0..self.local_editable_end],
            collection: t,
        }
    }

    pub fn iter_remote_editable<'a, I>(&'a self, t: &'a [I]) -> LineIter<'a, L, I> {
        LineIter {
            bricks: &self.idx_buf[self.local_editable_end..self.remote_editable_end],
            collection: t,
        }
    }

    pub fn iter_local_editable_mut<'a, 'b, I>(
        &'a self,
        t: &'b mut [I],
    ) -> LineIterMut<'a, 'b, L, I> {
        LineIterMut {
            bricks: &self.idx_buf[0..self.local_editable_end],
            col_ptr: NonNull::from(t),
            _cpd: PhantomData,
        }
    }

    pub fn iter_remote_editable_mut<'a, 'b, I>(
        &'a self,
        t: &'b mut [I],
    ) -> LineIterMut<'a, 'b, L, I> {
        LineIterMut {
            bricks: &self.idx_buf[self.local_editable_end..self.remote_editable_end],
            col_ptr: NonNull::from(t),
            _cpd: PhantomData,
        }
    }

    /// Iterate through the historical items, from NEWEST to OLDEST
    pub fn iter_history<'a, I>(&'a self, t: &'a [I]) -> LineIter<'a, L, I> {
        LineIter {
            bricks: &self.idx_buf[self.remote_editable_end..self.history_end],
            collection: t,
        }
    }

    pub fn pop_local_editable_front(&mut self) {
        //        0 LE1 => LE2
        //        1 LE2 => RE1            < LEND
        // > LEND 2 RE1 => RE2
        //        3 RE2 => HI1            < REND
        // > REND 4 HI1 => HI2
        //        5 HI2 => HI3
        //        6 HI3 => LE1 (now XX1)  < HEND
        // > HEND 7 XX1 => XX1 (now XX2)
        if self.local_editable_end == 0 {
            return;
        }
        rot_left(&mut self.idx_buf[..self.history_end]);
        self.local_editable_end -= 1;
        self.remote_editable_end -= 1;
        self.history_end -= 1;
    }

    pub fn pop_remote_editable_front(&mut self) {
        //        0 LE1 => LE1
        //        1 LE2 => LE2
        // > LEND 2 RE1 => RE2            < LEND
        //        3 RE2 => HI1            < REND
        // > REND 4 HI1 => HI2
        //        5 HI2 => HI3
        //        6 HI3 => RE1 (now XX1)  < HEND
        // > HEND 7 XX1 => XX1 (now XX2)

        if self.remote_editable_end == self.local_editable_end {
            return;
        }
        rot_left(&mut self.idx_buf[self.local_editable_end..self.history_end]);
        self.remote_editable_end -= 1;
        self.history_end -= 1;
    }

    pub fn local_editable_front(&self) -> Option<usize> {
        if self.local_editable_end == 0 {
            None
        } else {
            Some(self.idx_buf[0])
        }
    }

    pub fn remote_editable_front(&self) -> Option<usize> {
        if self.remote_editable_end == self.local_editable_end {
            None
        } else {
            Some(self.idx_buf[self.local_editable_end])
        }
    }

    // Operations:
    //
    // * Insert user editable -> Fails if all items already UE
    // * Insert inco editable -> Fails if all items already UE + IE
    // * Insert history       -> Fails if all items already UE + IE (not + history!)
    pub fn insert_local_editable_front(&mut self) -> Result<usize, ()> {
        if self.local_editable_end == L {
            return Err(());
        }
        // Rotate in at least one free/history
        let end = self.history_end.wrapping_add(1).min(L);
        rot_right(&mut self.idx_buf[..end]);
        self.local_editable_end = self.local_editable_end.wrapping_add(1).min(L);
        self.remote_editable_end = self.remote_editable_end.wrapping_add(1).min(L);
        self.history_end = self.history_end.wrapping_add(1).min(L);
        Ok(self.idx_buf[0])
    }

    pub fn insert_remote_editable_front(&mut self) -> Result<usize, ()> {
        if self.remote_editable_end == L {
            return Err(());
        }
        // Rotate in at least one free/history
        let end = self.history_end.wrapping_add(1).min(L);
        rot_right(&mut self.idx_buf[self.local_editable_end..end]);
        self.remote_editable_end = self.remote_editable_end.wrapping_add(1).min(L);
        self.history_end = self.history_end.wrapping_add(1).min(L);
        Ok(self.idx_buf[self.local_editable_end])
    }

    pub fn submit_local_editable(&mut self) {
        // We want to swap ue and ie regions.
        let range = &mut self.idx_buf[..self.remote_editable_end];

        // TODO(AJM): This is memory-friendly (requires only 1xusize extra),
        // but VERY CPU-unfriendly O(n^2) copies. This can be mitigated by
        // keeping the number of inco_editable items low, ideally 0/1.
        //
        // Alternatively, I could use O(n) extra storage, and assemble
        // the output directly.
        for _ in 0..(self.remote_editable_end - self.local_editable_end) {
            rot_right(range);
        }
        self.remote_editable_end -= self.local_editable_end;
        self.local_editable_end = 0;
    }

    pub fn submit_remote_editable(&mut self) {
        self.remote_editable_end = self.local_editable_end;
    }
}

#[cfg(test)]
pub mod brick_tests {
    use super::Bricks;

    #[test]
    fn smoke() {
        let mut brick = Bricks::<8>::new();
        println!("{:?}", brick);
        for i in 0..8 {
            let x = brick.insert_local_editable_front().unwrap();
            println!("{:?}", brick);
            assert_eq!(x, i);
        }
        println!("{:?}", brick);
        brick.insert_local_editable_front().unwrap_err();
        assert_eq!(
            brick,
            Bricks {
                idx_buf: [7, 6, 5, 4, 3, 2, 1, 0],
                local_editable_end: 8,
                remote_editable_end: 8,
                history_end: 8,
            }
        );
        println!("=====");
        let mut brick = Bricks::<8>::new();
        for i in 0..4 {
            let x = brick.insert_local_editable_front().unwrap();
            println!("{:?}", brick);
            assert_eq!(x, i);
        }
        assert_eq!(
            brick,
            Bricks {
                idx_buf: [3, 2, 1, 0, 4, 5, 6, 7],
                local_editable_end: 4,
                remote_editable_end: 4,
                history_end: 4,
            }
        );
        println!("-----");
        for i in 4..8 {
            let x = brick.insert_remote_editable_front().unwrap();
            println!("{:?}", brick);
            assert_eq!(x, i);
        }
        assert_eq!(
            brick,
            Bricks {
                idx_buf: [3, 2, 1, 0, 7, 6, 5, 4],
                local_editable_end: 4,
                remote_editable_end: 8,
                history_end: 8,
            }
        );
        println!("{:?}", brick);
        println!("=====");
        let mut brick = Bricks::<8>::new();
        for i in 0..3 {
            let x = brick.insert_local_editable_front().unwrap();
            println!("{:?}", brick);
            assert_eq!(x, i);
        }
        for i in 3..5 {
            let x = brick.insert_remote_editable_front().unwrap();
            println!("{:?}", brick);
            assert_eq!(x, i);
        }
        println!("{:?}", brick);
        assert_eq!(
            brick,
            Bricks {
                idx_buf: [2, 1, 0, 4, 3, 5, 6, 7],
                local_editable_end: 3,
                remote_editable_end: 5,
                history_end: 5,
            }
        );
        println!("-----");
        brick.submit_local_editable();
        println!("{:?}", brick);
        assert_eq!(
            brick,
            Bricks {
                idx_buf: [4, 3, 2, 1, 0, 5, 6, 7],
                local_editable_end: 0,
                remote_editable_end: 2,
                history_end: 5,
            }
        );
        println!("-----");
        brick.submit_remote_editable();
        println!("{:?}", brick);
        assert_eq!(
            brick,
            Bricks {
                idx_buf: [4, 3, 2, 1, 0, 5, 6, 7],
                local_editable_end: 0,
                remote_editable_end: 0,
                history_end: 5,
            }
        );
        println!("=====");
        for i in 5..8 {
            let x = brick.insert_local_editable_front().unwrap();
            println!("{:?}", brick);
            assert_eq!(x, i);
        }
        println!("{:?}", brick);
        assert_eq!(
            brick,
            Bricks {
                idx_buf: [7, 6, 5, 4, 3, 2, 1, 0],
                local_editable_end: 3,
                remote_editable_end: 3,
                history_end: 8,
            }
        );
        println!("-----");
        for i in 0..2 {
            let x = brick.insert_remote_editable_front().unwrap();
            println!("{:?}", brick);
            assert_eq!(x, i);
        }
        println!("{:?}", brick);
        assert_eq!(
            brick,
            Bricks {
                idx_buf: [7, 6, 5, 1, 0, 4, 3, 2],
                local_editable_end: 3,
                remote_editable_end: 5,
                history_end: 8,
            }
        );

        let mut buf = [10, 20, 30, 40, 50, 60, 70, 80];
        assert_eq!(
            brick
                .iter_local_editable(&buf)
                .copied()
                .collect::<Vec<_>>()
                .as_slice(),
            &[80, 70, 60],
        );
        assert_eq!(
            brick
                .iter_local_editable_mut(&mut buf)
                .map(|c| *c)
                .collect::<Vec<_>>()
                .as_slice(),
            &[80, 70, 60],
        );
        assert_eq!(
            brick
                .iter_remote_editable(&buf)
                .copied()
                .collect::<Vec<_>>()
                .as_slice(),
            &[20, 10],
        );
        assert_eq!(
            brick
                .iter_remote_editable_mut(&mut buf)
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
            let x = brick.insert_local_editable_front().unwrap();
            println!("{:?}", brick);
            assert_eq!(x, i);
        }
        println!("{:?}", brick);
        assert_eq!(
            brick,
            Bricks {
                idx_buf: [4, 3, 2, 7, 6, 5, 1, 0],
                local_editable_end: 6,
                remote_editable_end: 8,
                history_end: 8,
            }
        );
        brick.insert_remote_editable_front().unwrap_err();
        assert_eq!(brick.insert_local_editable_front().unwrap(), 0);
    }
}
