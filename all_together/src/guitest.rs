use std::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    thread,
};

use classify_dots::DotLocationInfo;
use eframe::{
    egui::{
        mutex::RwLock, Color32, ColorImage, ComboBox, Image, Sense, SidePanel, Slider,
        TextureHandle, TextureOptions, TopBottomPanel, ViewportBuilder,
    },
    epaint::ImageDelta,
    App, CreationContext, NativeOptions,
};
use log::info;

use crate::{
    camera_input::{self, CameraFrameInfo, CameraSettings, DeviceSelect},
    motor_thread::{motor_thread, MotorCommand, MotorState},
};

pub fn go() {
    let (frame_push, frame_recv) = channel();
    let (return_push, return_recv) = channel();
    let (device_select, device_sel_recv) = channel();
    let settings = Arc::new(RwLock::new(CameraSettings {
        dot_locations: DotLocationInfo {
            point_locations: vec![],
            dot_size: 10,
            brightness_thresh: 200,
        },
        is_actually_solved: false,
        is_auto_adjusting_brightness: false,
        path: None,
        available_cams: (Vec::new(), Vec::new()),
        is_camera_feed: false,
    }));

    let settings_clone = settings.clone();
    thread::spawn(move || {
        camera_input::process_thread(frame_push, return_recv, device_sel_recv, settings_clone);
    });

    let (mot_cmd_send, mot_cmd_recv) = channel();
    let mot_state = Arc::new(RwLock::new(MotorState::new()));
    let mot_state_clone = mot_state.clone();

    thread::spawn(move || {
        motor_thread(mot_cmd_recv, mot_state_clone);
    });

    let options = NativeOptions {
        viewport: ViewportBuilder::default().with_fullscreen(true),
        ..Default::default()
    };
    eframe::run_native(
        "Free Flow",
        options,
        Box::new(move |cc| {
            Box::new(MyApp::new(
                cc,
                frame_recv,
                return_push,
                device_select,
                settings,
                mot_state,
                mot_cmd_send,
            ))
        }),
    )
    .unwrap();
    std::process::exit(0);
}

pub struct MyApp {
    frame_recv: Receiver<CameraFrameInfo>,
    frame_return: Sender<CameraFrameInfo>,
    camera_select: Sender<DeviceSelect>,

    frame_texture: TextureHandle,

    heads_texture: TextureHandle,
    solved_texture: TextureHandle,
    motor_grid_texture: TextureHandle,

    camera_settings: Arc<RwLock<CameraSettings>>,

    motor_state: Arc<RwLock<MotorState>>,
    motor_command: Sender<MotorCommand>,

    mot_tl: Option<(u32, u32)>,
    mot_tr: Option<(u32, u32)>,
    mot_bl: Option<(u32, u32)>,

    clicked_loc: Option<(u32, u32)>,

    head_x: u8,
    head_y: u8,
    selected_camera: usize,
}

impl MyApp {
    pub fn new(
        cc: &CreationContext,
        frame_recv: Receiver<CameraFrameInfo>,
        frame_return: Sender<CameraFrameInfo>,
        camera_select: Sender<DeviceSelect>,
        camera_settings: Arc<RwLock<CameraSettings>>,
        motor_state: Arc<RwLock<MotorState>>,
        motor_command: Sender<MotorCommand>,
    ) -> Self {
        Self {
            frame_recv,
            frame_return,
            camera_settings,
            motor_state,
            motor_command,
            camera_select,

            frame_texture: cc.egui_ctx.load_texture(
                "camera_frame",
                ColorImage::new([256, 256], Color32::BLACK),
                TextureOptions::NEAREST,
            ),
            heads_texture: cc.egui_ctx.load_texture(
                "heads_frame",
                ColorImage::new([5, 5], Color32::BLACK),
                TextureOptions::NEAREST,
            ),
            solved_texture: cc.egui_ctx.load_texture(
                "solved_frame",
                ColorImage::new([5, 5], Color32::BLACK),
                TextureOptions::NEAREST,
            ),
            motor_grid_texture: cc.egui_ctx.load_texture(
                "motor_frame",
                ColorImage::new([1, 1], Color32::GOLD),
                TextureOptions::NEAREST,
            ),

            head_x: 0,
            head_y: 0,

            mot_bl: None,
            mot_tl: None,
            mot_tr: None,
            clicked_loc: None,
            selected_camera: 0,
        }
    }
}

impl App for MyApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // handle new camera frames
        if let Ok(data) = self.frame_recv.try_recv() {
            let cam_img = ColorImage::from_rgba_premultiplied(
                [data.frame_width, data.frame_height],
                &data.frame,
            );
            ctx.tex_manager().write().set(
                self.frame_texture.id(),
                ImageDelta::full(cam_img, TextureOptions::NEAREST),
            );

            let heads_img = ColorImage::from_rgba_premultiplied([5, 5], &data.unsolved_grid.image);
            ctx.tex_manager().write().set(
                self.heads_texture.id(),
                ImageDelta::full(heads_img, TextureOptions::NEAREST),
            );

            let solved_img = ColorImage::from_rgba_premultiplied([5, 5], &data.solved_grid.image);
            ctx.tex_manager().write().set(
                self.solved_texture.id(),
                ImageDelta::full(solved_img, TextureOptions::NEAREST),
            );

            self.frame_return.send(data).unwrap();
        }

        TopBottomPanel::top("camera_feed_frame").show(ctx, |ui| {
            ui.columns(3, |ui| {
                ui[0].heading("Camera Feed");
                // draw camera frame
                let response = ui[0].add(
                    Image::from_texture(&self.frame_texture)
                        .shrink_to_fit()
                        .sense(Sense::click()),
                );
                if response.clicked() && self.head_y < 5 {
                    if let Some(position) = response.interact_pointer_pos() {
                        let position = position - response.rect.left_top();
                        let click_x = position.x / response.rect.width();
                        let click_y = position.y / response.rect.height();

                        let click_x = (click_x * self.frame_texture.size()[0] as f32) as i32;
                        let click_y = (click_y * self.frame_texture.size()[1] as f32) as i32;
                        let click_x =
                            click_x.max(0).min(self.frame_texture.size()[0] as i32 - 1) as u32;
                        let click_y =
                            click_y.max(0).min(self.frame_texture.size()[1] as i32 - 1) as u32;

                        self.camera_settings
                            .write()
                            .dot_locations
                            .point_locations
                            .push(([click_x, click_y], [self.head_x, self.head_y]));

                        self.head_x += 1;
                        if self.head_x == 5 {
                            self.head_x = 0;
                            self.head_y += 1;
                        }
                    }
                }

                ui[1].heading("Flow Free Heads Detected");
                ui[1].add(Image::from_texture(&self.heads_texture).shrink_to_fit());

                ui[2].heading("Flow Free Solved");
                ui[2].add(Image::from_texture(&self.solved_texture).shrink_to_fit());
            });

            ui.add_space(40.);
        });

        SidePanel::left("cam_controls").show(ctx, |ui| {
            if !self.camera_settings.read().is_camera_feed {
                let available_cams = &self.camera_settings.read().available_cams;
                if available_cams.0.len() == 0 {
                    ui.label("No available devices");
                    return;
                }

                if available_cams.0.len() <= self.selected_camera {
                    self.selected_camera = 0;
                }

                ComboBox::from_label("Select camera device").show_index(
                    ui,
                    &mut self.selected_camera,
                    available_cams.0.len(),
                    |index| &available_cams.0[index],
                );

                if ui.button("Refresh list").clicked() {
                    self.camera_select.send(DeviceSelect::RefreshList).unwrap();
                } else if ui.button("Open selected device").clicked() {
                    self.camera_select
                        .send(DeviceSelect::Select(
                            available_cams.1[self.selected_camera].clone(),
                        ))
                        .unwrap();
                }
                return;
            }

            if ui.button("Back to list").clicked() {
                self.camera_select.send(DeviceSelect::RefreshList).unwrap();
            }

            let settings = self.camera_settings.read();
            let mut brightness_thresh = settings.dot_locations.brightness_thresh;
            let mut dot_size = settings.dot_locations.dot_size;
            drop(settings);

            let result =
                ui.add(Slider::new(&mut brightness_thresh, 1..=1000).text("Brightness thresh"));
            if result.changed() {
                self.camera_settings.write().dot_locations.brightness_thresh = brightness_thresh;
            }

            if !self.camera_settings.read().is_auto_adjusting_brightness {
                let result = ui.button("Auto threshold");
                if result.clicked() {
                    let mut settings = self.camera_settings.write();
                    settings.is_auto_adjusting_brightness = true;
                }
            } else {
                let result = ui.button("Cancel auto threshhold");
                if result.clicked() {
                    let mut settings = self.camera_settings.write();
                    settings.is_auto_adjusting_brightness = false;
                }
            }

            ui.label(format!(
                "Is solved: {}",
                self.camera_settings.read().path.is_some()
            ));

            let result = ui.add(Slider::new(&mut dot_size, 0..=20).text("Dot size"));
            if result.changed() {
                self.camera_settings.write().dot_locations.dot_size = dot_size;
            }

            if ui.button("Clear points").clicked() {
                self.camera_settings
                    .write()
                    .dot_locations
                    .point_locations
                    .clear();
                self.head_x = 0;
                self.head_y = 0;
            }
        });

        let mut is_fully_calibrated = false;
        let is_executing = self.motor_state.read().is_executing;

        SidePanel::right("mot_controls").show(ctx, |ui| {
            eframe::egui::containers::scroll_area::ScrollArea::vertical().show(ui, |ui| {
                if !self.motor_state.read().file_open {
                    if ui.button("Try motor connect").clicked() {
                        self.motor_command.send(MotorCommand::Wakeup).unwrap();
                    }
                    return;
                } else if !self.motor_state.read().connect_active {
                    ui.label("Connecting...");
                    return;
                }

                is_fully_calibrated = self.motor_state.read().has_calibrated;

                if is_executing {
                    ui.label("Robot executing...");
                } else {
                    ui.label("Robot idle");
                }

                ui.label(format!(
                    "Motor last state: {:?}",
                    self.motor_state.read().last_response
                ));

                ui.add_enabled_ui(!is_executing, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Home").clicked() {
                            self.motor_command.send(MotorCommand::Home).unwrap();
                            info!("Home!");
                        }
                        if ui.button("Pen up").clicked() {
                            self.motor_command.send(MotorCommand::PenUp).unwrap();
                            info!("Pen up!");
                        }
                        if ui.button("Pen down").clicked() {
                            self.motor_command.send(MotorCommand::PenDown).unwrap();
                            info!("Pen down!");
                        }
                    });

                    if !self.motor_state.read().has_homed {
                        return;
                    }

                    let size = self.motor_state.read().size;
                    ui.label(format!("Detected grid size: ({}, {})", size.0, size.1));

                    let response = ui.add(
                        Image::new(&self.motor_grid_texture)
                            .shrink_to_fit()
                            .sense(Sense::click()),
                    );
                    if response.clicked() {
                        if let Some(position) = response.interact_pointer_pos() {
                            let position = position - response.rect.left_top();
                            let click_x =
                                size.0 as f32 * (position.x.max(0.) as f32) / response.rect.width();
                            let click_y = size.1 as f32 * (position.y.max(0.) as f32)
                                / response.rect.height();

                            println!("Click at {click_x}, {click_y}");

                            self.clicked_loc = Some((click_x as _, click_y as _));

                            self.motor_command
                                .send(MotorCommand::MoveTo(click_x as u32, click_y as u32))
                                .unwrap();
                        }
                    }

                    ui.horizontal(|ui| {
                        let was_enabled = ui.is_enabled();
                        ui.set_enabled(self.clicked_loc.is_some());
                        if ui.button("Set TL").clicked() {
                            self.mot_tl = self.clicked_loc;
                        }
                        if ui.button("Set TR").clicked() {
                            self.mot_tr = self.clicked_loc;
                        }
                        if ui.button("Set BL").clicked() {
                            self.mot_bl = self.clicked_loc;
                        }
                        ui.set_enabled(was_enabled);

                        let points: Option<_> = try { (self.mot_tl?, self.mot_tr?, self.mot_bl?) };
                        ui.add_enabled_ui(points.is_some(), |ui| {
                            if ui.button("Calibrate").clicked() {
                                let points = points.unwrap();
                                self.motor_command
                                    .send(MotorCommand::Calibrate(points.0, points.1, points.2))
                                    .unwrap();
                            }
                        });
                    });
                });
            });
        });

        if is_fully_calibrated {
            SidePanel::right("motor_grid_controls").show(ctx, |ui| {
                ui.add_enabled_ui(!is_executing, |ui| {
                    for y in 0..5 {
                        ui.horizontal(|ui| {
                            for x in 0..5 {
                                if ui.button(format!("{x}, {y}")).clicked() {
                                    self.motor_command
                                        .send(MotorCommand::MoveToGrid(x, y))
                                        .unwrap();
                                }
                            }
                        });
                    }

                    if let Some(path) = &self.camera_settings.read().path {
                        if ui.button("ACCIO ROBOT GO OF DOOOOOM").clicked() {
                            self.motor_command
                                .send(MotorCommand::MotorExecute(path.clone()))
                                .unwrap();
                        }
                    }
                });
            });
        }

        ctx.request_repaint();
    }
}
