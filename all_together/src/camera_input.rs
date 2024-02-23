use std::sync::{
    mpsc::{Receiver, Sender},
    Arc,
};

use classify_dots::DotLocationInfo;
use eframe::egui::mutex::RwLock;
use log::{error, warn};
use pathfind::Instruction;
use read_cam::{Cam, CamError, CamIndex, MyCamera, RgbaView};

use crate::grid_representation::GridRepresentation;

pub struct CameraFrameInfo {
    pub frame: Vec<u8>,
    pub frame_width: usize,
    pub frame_height: usize,

    pub unsolved_grid: GridRepresentation,
    pub solved_grid: GridRepresentation,
}

pub struct CameraSettings {
    pub dot_locations: DotLocationInfo,
    pub is_actually_solved: bool,
    pub is_auto_adjusting_brightness: bool,
    pub path: Option<Vec<Instruction>>,
    pub available_cams: (Vec<String>, Vec<CamIndex>),
    pub is_camera_feed: bool,
}

pub enum DeviceSelect {
    Select(CamIndex),
    RefreshList,
}

pub fn process_thread(
    push_stack: Sender<CameraFrameInfo>,
    pull_stack: Receiver<CameraFrameInfo>,
    device_select: Receiver<DeviceSelect>,
    settings: Arc<RwLock<CameraSettings>>,
) {
    let _ = run_process_thread(push_stack, pull_stack, device_select, settings);
}

pub fn run_process_thread(
    push_stack: Sender<CameraFrameInfo>,
    pull_stack: Receiver<CameraFrameInfo>,
    device_select: Receiver<DeviceSelect>,
    settings: Arc<RwLock<CameraSettings>>,
) -> Result<(), ()> {
    loop {
        settings.write().is_camera_feed = false;
        let devices = match Cam::enumerate_devices() {
            Ok(x) => x,
            Err(e) => {
                error!("Could not enumerate camera devices: {e}");
                error!("Goodbye from camera thread...");
                return Err(());
            }
        };

        settings.write().available_cams = devices;

        let selected_device = match device_select.recv() {
            Ok(DeviceSelect::Select(dev)) => dev,
            Ok(_) => continue,
            Err(_) => return Err(()),
        };

        let cam: Result<Cam, CamError> = try {
            let mut cam = Cam::new(selected_device)?;
            cam.open()?;
            cam
        };

        let mut cam = match cam {
            Ok(cam) => cam,
            Err(e) => {
                warn!("Failed to open camera: {}", e);
                continue;
            }
        };

        let mut info = CameraFrameInfo {
            frame: vec![0; (cam.width() * cam.height() * 4) as _],
            frame_width: 0,
            frame_height: 0,

            unsolved_grid: GridRepresentation::empty(),
            solved_grid: GridRepresentation::empty(),
        };

        settings.write().is_camera_feed = true;
        // info!("Camera stream opened ({}x{})", cam.width(), cam.height());

        loop {
            if let Ok(DeviceSelect::RefreshList) = device_select.try_recv() {
                break;
            }

            match cam.raw_frame(&mut info.frame) {
                Ok(_) => (),
                Err(e) => {
                    warn!("Failed to get camera frame: {e}");
                    break;
                }
            }
            info.frame_width = cam.width() as _;
            info.frame_height = cam.height() as _;

            let mut rgb = RgbaView::new(&mut info.frame, info.frame_width, info.frame_height);

            let (head_locs, heads) =
                classify_dots::get_map_layout(&settings.read().dot_locations, &mut rgb);

            let (solved_grid, is_solved) = solver::solve(head_locs.clone(), heads);

            if settings.read().is_auto_adjusting_brightness && !is_solved {
                let mut settings = settings.write();
                if settings.dot_locations.brightness_thresh < 20 {
                    settings.is_auto_adjusting_brightness = false;
                } else {
                    settings.dot_locations.brightness_thresh -= 1;
                }
            } else if is_solved && settings.read().is_auto_adjusting_brightness {
                let mut settings = settings.write();
                settings.is_auto_adjusting_brightness = false;
                settings.dot_locations.brightness_thresh -= 20;
            }

            settings.write().path = if is_solved && !solved_grid.contains_zeroes() {
                let path = pathfind::pathfind(head_locs.clone(), &solved_grid);
                Some(path)
            } else {
                None
            };

            info.unsolved_grid.update(&head_locs);
            info.solved_grid.update(&solved_grid);
            settings.read().dot_locations.draw_dots(&mut rgb);

            // push - pull
            push_stack.send(info).map_err(|_| ())?;
            info = pull_stack.recv().map_err(|_| ())?;
        }
    }
}
