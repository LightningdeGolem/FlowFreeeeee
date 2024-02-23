use std::{thread, time::Duration};

use motor_control::Motor;

fn main() {
    let mut motor = Motor::new("/dev/ttyACM0").unwrap();
    motor.wait_for_ready().unwrap();
}
