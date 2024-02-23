mod my_grid;
pub use my_grid::*;

#[inline]
pub fn get_around(p: (IndexTy, IndexTy)) -> [(IndexTy, IndexTy); 4] {
    [(p.0 - 1, p.1), (p.0, p.1 - 1), (p.0 + 1, p.1), (p.0, p.1 + 1)]
}