use solver::Array2D;

const COLS: [[u8; 3]; 8] = [
    [255, 0, 0],
    [0, 0, 255],
    [0, 255, 0],
    [255, 255, 0],
    [255, 0, 255],
    [0, 255, 255],
    [125, 255, 125],
    [125, 255, 125],
];

pub struct GridRepresentation {
    pub image: [u8; 4 * 25],
}

impl GridRepresentation {
    pub fn empty() -> Self {
        Self { image: [0; 25 * 4] }
    }
    pub fn update(&mut self, grid: &Array2D) {
        for (i, b) in self.image.chunks_mut(4).enumerate() {
            let x = (i % 5) as isize;
            let y = (i / 5) as isize;

            let col = grid[(x, y)];

            let col = match col {
                0 => [0, 0, 0],
                255 => [255, 255, 255],
                col => COLS[col as usize % COLS.len()],
            };
            let col = [col[0], col[1], col[2], 100];

            b.copy_from_slice(&col);
        }
    }
}
