#![feature(try_blocks)]

mod camera_input;
mod gui;
mod guitest;

mod grid_representation;
mod motor_thread;

fn main() {
    env_logger::init();
    guitest::go();
}
