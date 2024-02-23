use read_cam::Image;
use solver::Array2D;

pub struct DotLocationInfo {
    /// Maps between coordinates on screen to coordinates on grid
    pub point_locations: Vec<([u32; 2], [u8; 2])>,
    pub dot_size: u32,
    pub brightness_thresh: u32,
}

impl DotLocationInfo {
    pub fn draw_dots(&self, img: &mut impl Image) {
        for (point, _) in self.point_locations.iter() {
            img.draw_rect(self.dot_size, point[0], point[1], [255, 255, 255]);
        }
    }
}

fn pythag(a: [u8; 3], b: [u8; 3]) -> i32 {
    (a[0] as i32 - b[0] as i32).pow(2)
        + (a[1] as i32 - b[1] as i32).pow(2)
        + (a[2] as i32 - b[2] as i32).pow(2)
}

fn brightness(x: [u8; 3]) -> u32 {
    x[0] as u32 + x[1] as u32 + x[2] as u32
}

pub fn get_map_layout(
    info: &DotLocationInfo,
    img: &impl read_cam::Image,
) -> (Array2D, Vec<((isize, isize), (isize, isize))>) {
    let mut rgbs: Vec<_> = info
        .point_locations
        .iter()
        .enumerate()
        .map(|(i, (img_coord, _))| {
            let size = info.dot_size;
            let mut col = [0; 3];
            for x in (img_coord[0] - size)..(img_coord[0] + size) {
                for y in (img_coord[1] - size)..(img_coord[1] + size) {
                    let px = img.read_pixel(x, y);

                    col[0] += px[0] as u32;
                    col[1] += px[1] as u32;
                    col[2] += px[2] as u32;
                }
            }
            col[0] /= size * size * 4;
            col[1] /= size * size * 4;
            col[2] /= size * size * 4;

            let col = [col[0] as u8, col[1] as u8, col[2] as u8];
            // println!("bright: {}", brightness(col));
            (i, col)
        })
        .filter(|(_, x)| brightness(*x) > info.brightness_thresh)
        .collect();

    let mut grid = Array2D::new(5, 5);
    let mut pairs = Vec::new();
    let mut id = 1;
    loop {
        if rgbs.len() < 2 {
            break;
        }
        let last_elem = *rgbs.last().unwrap();

        let (removal_index, &pair_index, _) = rgbs
            .iter()
            .enumerate()
            .take(rgbs.len() - 1)
            .map(|(j, (i, x))| (j, i, pythag(*x, last_elem.1)))
            .min_by_key(|x| x.2)
            .unwrap();

        rgbs.remove(removal_index);
        rgbs.pop();

        let locs = (
            info.point_locations[last_elem.0].1,
            info.point_locations[pair_index].1,
        );

        grid[(locs.0[0] as _, locs.0[1] as _)] = id;
        grid[(locs.1[0] as _, locs.1[1] as _)] = id;
        pairs.push((
            (locs.0[0] as _, locs.0[1] as _),
            (locs.1[0] as _, locs.1[1] as _),
        ));
        id += 1;
    }
    if let Some(leftover) = rgbs.get(0) {
        let locs = info.point_locations[leftover.0].1;
        grid[(locs[0] as _, locs[1] as _)] = 0xFF;
    }

    (grid, pairs)
}
