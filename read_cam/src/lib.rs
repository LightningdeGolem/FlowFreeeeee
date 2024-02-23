#![feature(vec_into_raw_parts)]

use nokhwa::{
    pixel_format::RgbAFormat,
    utils::{CameraFormat, FrameFormat, RequestedFormat, RequestedFormatType},
    Camera,
};

pub use nokhwa::utils::CameraIndex as CamIndex;
pub use nokhwa::NokhwaError as CamError;

pub trait Image {
    fn read_pixel(&self, x: u32, y: u32) -> [u8; 3];
    fn write_pixel(&mut self, x: u32, y: u32, val: [u8; 3]);
    fn width(&self) -> usize;
    fn height(&self) -> usize;

    fn draw_rect(&mut self, half_rad: u32, cx: u32, cy: u32, col: [u8; 3]) {
        for x in (cx - half_rad)..(cx + half_rad) {
            for y in (cy - half_rad)..(cy + half_rad) {
                self.write_pixel(x, y, col);
            }
        }
    }
}

pub struct RgbaView<'a> {
    buf: &'a mut [u8],
    width: usize,
    height: usize,
}

impl<'a> RgbaView<'a> {
    pub fn new(buf: &'a mut [u8], width: usize, height: usize) -> Self {
        Self { buf, width, height }
    }
}

impl<'a> Image for RgbaView<'a> {
    fn read_pixel(&self, x: u32, y: u32) -> [u8; 3] {
        if x >= self.width as _ || y >= self.height as _ {
            return [0, 0, 0];
        }
        let index = (self.width * y as usize + x as usize) * 4;
        [self.buf[index], self.buf[index + 1], self.buf[index + 2]]
    }

    fn write_pixel(&mut self, x: u32, y: u32, val: [u8; 3]) {
        if x >= self.width as _ || y >= self.height as _ {
            return;
        }

        let index = (self.width * y as usize + x as usize) * 4;
        self.buf[index..index + 3].copy_from_slice(&val);
    }

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }
}

pub trait MyCamera {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn open(&mut self) -> Result<(), CamError>;
    fn raw_frame(&mut self, buf: &mut [u8]) -> Result<(), CamError>;
}

pub struct Cam {
    camera: Camera,
}

impl Cam {
    fn pick_format(mut formats: Vec<CameraFormat>) -> Option<CameraFormat> {
        formats.sort_by(|x, y| y.frame_rate().cmp(&x.frame_rate()));
        for format in formats {
            match format.format() {
                FrameFormat::MJPEG | FrameFormat::RAWRGB => return Some(format),
                _ => (),
            }
        }
        None
    }
    pub fn new(index: CamIndex) -> Result<Self, CamError> {
        let mut camera = Camera::new(
            index,
            RequestedFormat::new::<RgbAFormat>(RequestedFormatType::None),
        )?;
        if let Some(format) = Self::pick_format(camera.compatible_camera_formats()?) {
            camera.set_camera_requset(RequestedFormat::new::<RgbAFormat>(
                RequestedFormatType::Exact(format),
            ))?;
        } else {
            println!("Couldn't find format that was gd :(");
        }
        println!("{:?}", camera.camera_format());

        Ok(Self { camera })
    }

    pub fn enumerate_devices() -> Result<(Vec<String>, Vec<CamIndex>), CamError> {
        let devs = nokhwa::query(nokhwa::utils::ApiBackend::Auto)?;
        Ok(devs
            .into_iter()
            .map(|info| (info.human_name(), info.index().clone()))
            .unzip())
    }
}

impl MyCamera for Cam {
    fn width(&self) -> u32 {
        self.camera.resolution().width_x
    }
    fn height(&self) -> u32 {
        self.camera.resolution().height_y
    }

    fn open(&mut self) -> Result<(), CamError> {
        self.camera.open_stream()
    }

    fn raw_frame(&mut self, buf: &mut [u8]) -> Result<(), CamError> {
        self.camera.write_frame_to_buffer::<RgbAFormat>(buf)?;

        Ok(())
    }
}
