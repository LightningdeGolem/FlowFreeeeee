use classify_dots::DotLocationInfo;
use minifb::{Key, Window, WindowOptions};
use read_cam::{Cam, Frame, Image, ImageCam, MyCamera};
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

pub struct MiniFrame {
    top_left: (u32, u32),
    bot_right: (u32, u32),
}

impl MiniFrame {
    pub fn new(top_left: (u32, u32), size: (u32, u32)) -> Self {
        Self {
            top_left,
            bot_right: (top_left.0 + size.0, top_left.1 + size.1),
        }
    }
}

pub struct MiniFrameWriter<'a, 'b, T> {
    parent: &'a mut T,
    frame: &'b MiniFrame,
}

impl<'a, 'b, T> Image for MiniFrameWriter<'a, 'b, T>
where
    T: Image,
{
    fn read_pixel(&self, x: u32, y: u32) -> [u8; 3] {
        self.parent
            .read_pixel(x + self.frame.top_left.0, y + self.frame.top_left.1)
    }

    fn write_pixel(&mut self, x: u32, y: u32, val: [u8; 3]) {
        self.parent
            .write_pixel(x + self.frame.top_left.0, y + self.frame.top_left.1, val)
    }

    fn width(&self) -> usize {
        (self.frame.bot_right.0 - self.frame.top_left.0) as _
    }

    fn height(&self) -> usize {
        (self.frame.bot_right.1 - self.frame.top_left.1) as _
    }

    fn draw_rect(&mut self, half_rad: u32, cx: u32, cy: u32, col: [u8; 3]) {
        self.parent.draw_rect(
            half_rad,
            cx + self.frame.top_left.0,
            cy + self.frame.top_left.1,
            col,
        );
    }
}

fn draw_flow_free(img: &mut impl Image, grid: &Array2D) {
    assert_eq!(img.width(), img.height());
    let half_rad = img.width() / 10;
    let rad = img.width() / 5;

    for (x, xi) in (0..5).map(|x| (half_rad + rad * x, x)) {
        for (y, yi) in (0..5).map(|x| ((half_rad + rad * x), x)) {
            let id = grid[(xi as isize, yi as isize)];
            let col = match id {
                0 => [0, 0, 0],
                0xFF => [0xFF, 0xFF, 0xFF],
                id => COLS[id as usize % COLS.len()],
            };
            img.draw_rect(half_rad as u32, x as u32, y as u32, col);
        }
    }
}

fn remove_255s(grid: &Array2D) -> Array2D {
    let mut new_grid = grid.clone();
    for x in 0..5 {
        for y in 0..5 {
            if new_grid[(x, y)] == 0xFF {
                new_grid[(x, y)] = 0;
            }
        }
    }
    new_grid
}

fn main() {
    println!("Hello, world!");
    // let mut cam = Cam::new();
    let mut cam =
        ImageCam::new("/home/ollie/programming/flowfreeeee/datasets/yolo_train/images/0-1.bmp");
    let mut cam_display_window = Window::new(
        "Test",
        cam.width() as _,
        cam.height() as _,
        WindowOptions::default(),
    )
    .expect("Could not open window");

    let mut flow_free_grid_win = Window::new("Flow free grid", 250, 250, WindowOptions::default())
        .expect("Could not open window");

    let mut solved_flow_free_grid_win = Window::new(
        "Flow free grid - solved",
        250,
        250,
        WindowOptions::default(),
    )
    .expect("Could not open window");

    let mut frame = cam.new_empty_frame();
    let mut flow_free_frame_grid_frame = Frame::new(250, 250);
    let mut solved_flow_free_frame_grid_frame = Frame::new(250, 250);

    cam.open();

    let mut cooldown = false;
    let mut dot_locs = DotLocationInfo {
        point_locations: Vec::new(),
        dot_size: 6,
        brightness_thresh: 600,
    };

    let mut auto_brightness_adjust = false;

    let mut mouse_x = 0;
    let mut mouse_y = 0;
    while cam_display_window.is_open()
        && flow_free_grid_win.is_open()
        && solved_flow_free_grid_win.is_open()
    {
        cam.frame(&mut frame);

        if cam_display_window.get_mouse_down(minifb::MouseButton::Left) {
            if !cooldown {
                if let Some((x, y)) = cam_display_window.get_mouse_pos(minifb::MouseMode::Discard) {
                    dot_locs
                        .point_locations
                        .push(([x as u32, y as u32], [mouse_x, mouse_y]));
                    mouse_x += 1;
                    if mouse_x == 5 {
                        mouse_y += 1;
                        mouse_x = 0;
                    }

                    cooldown = true;
                }
            }
        } else if cooldown {
            cooldown = false;
        }

        if cam_display_window.is_key_down(Key::Q) {
            dot_locs.brightness_thresh += 1;
            println!("{}", dot_locs.brightness_thresh);
        } else if cam_display_window.is_key_down(Key::A) {
            dot_locs.brightness_thresh -= 1;
            println!("{}", dot_locs.brightness_thresh);
        }

        if cam_display_window.is_key_down(Key::Z) {
            auto_brightness_adjust = true;
            dot_locs.brightness_thresh = 600;
        }

        let (grid, heads) = classify_dots::get_map_layout(&dot_locs, &frame);

        let grid_to_solve = remove_255s(&grid);
        let (solved_grid, is_fully_solved) = solver::solve(grid_to_solve.clone(), heads);
        let path = pathfind::pathfind(grid.clone(), &solved_grid);

        if auto_brightness_adjust && is_fully_solved {
            dot_locs.brightness_thresh -= 20;
            auto_brightness_adjust = false;
            println!("Auto adjust finished: {}", dot_locs.brightness_thresh);
        } else if auto_brightness_adjust {
            dot_locs.brightness_thresh -= 1;
            println!("Auto adjust: {}", dot_locs.brightness_thresh);
        }

        if cam_display_window.is_key_down(Key::Space) {
            println!("{path:?}");
        }

        draw_flow_free(&mut flow_free_frame_grid_frame, &grid);
        draw_flow_free(&mut solved_flow_free_frame_grid_frame, &solved_grid);

        flow_free_grid_win
            .update_with_buffer(&flow_free_frame_grid_frame.data, 250, 250)
            .unwrap();
        solved_flow_free_grid_win
            .update_with_buffer(&solved_flow_free_frame_grid_frame.data, 250, 250)
            .unwrap();

        dot_locs.draw_dots(&mut frame);
        cam_display_window
            .update_with_buffer(&frame.data, cam.width() as _, cam.height() as _)
            .unwrap();
    }
}
