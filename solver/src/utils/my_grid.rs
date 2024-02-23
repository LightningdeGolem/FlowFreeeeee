use std::{alloc::{Layout, alloc_zeroed, dealloc}, ops::{Index, IndexMut}, fmt::Display};

pub type IndexTy = isize;


pub struct Array2D {
    data: *mut u8,
    width: usize,
    height: usize
}

impl Array2D {
    pub fn new(width: usize, height: usize) -> Array2D {
        let mem;
        unsafe {
            mem = alloc_zeroed(
                Layout::from_size_align(width * height, 1).unwrap()
            );
        }

        Self {
            data: mem,
            width, height
        }
    }

    #[track_caller]
    pub fn set_abs(&mut self, offset: usize, val: u8) {
        if offset >= self.width * self.height {
            panic!("Index out of range");
        }

        unsafe {
            *self.data.add(offset) = val;
        }
    }

    pub fn contains_zeroes(&self) -> bool {
        unsafe {
            for byte in core::slice::from_raw_parts(self.data, self.width * self.height) {
                if *byte == 0 {
                    return true;
                }
            }
        }
        false
    }

}

impl Clone for Array2D {
    fn clone(&self) -> Self {
        let new_data;
        unsafe {
            new_data = alloc_zeroed(Layout::from_size_align(self.width * self.height, 1).unwrap());
            core::ptr::copy(self.data, new_data, self.width * self.height);
        }
        Self { data: new_data, width: self.width.clone(), height: self.height.clone() }
    }
}

impl Display for Array2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for y in 0..self.height as IndexTy {
            for x in 0..self.width as IndexTy {
                let val = self[(x, y)];
                write!(f, "{val}")?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

impl Index<(IndexTy, IndexTy)> for Array2D {
    type Output = u8;

    fn index(&self, index: (IndexTy, IndexTy)) -> &Self::Output {
        if index.0 < 0 || index.0 >= self.width as IndexTy || index.1 < 0 || index.1 >= self.height as IndexTy {
            return &255;
        }
        unsafe {
            &*self.data.add(
                index.0 as usize +
                index.1 as usize * self.width
            )
        }
    }
}
impl IndexMut<(IndexTy, IndexTy)> for Array2D {
    fn index_mut(&mut self, index: (IndexTy, IndexTy)) -> &mut Self::Output {
        if index.0 < 0 || index.0 >= self.width as IndexTy || index.1 < 0 || index.1 > self.height as IndexTy {
            panic!("Attempt at mutable reference at out of bounds index in array 2d");
        }
        unsafe {
            &mut *self.data.add(
                index.0 as usize +
                index.1 as usize * self.width
            )
        }
    }
}

impl Drop for Array2D {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.data,Layout::from_size_align(self.width * self.height, 1).unwrap());
        }
    }
}