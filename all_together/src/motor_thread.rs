use std::sync::{
    mpsc::{Receiver, RecvError},
    Arc,
};

use eframe::egui::mutex::RwLock;
use log::warn;
use motor_control::{Motor, MotorResponse, SolvingCommand};
use pathfind::Instruction;

pub enum MotorCommand {
    Wakeup,
    MoveTo(u32, u32),
    MoveToGrid(u8, u8),
    Home,
    Calibrate((u32, u32), (u32, u32), (u32, u32)),
    MotorExecute(Vec<Instruction>),
    PenUp,
    PenDown,
    SetAutoPenup(bool),
    SetSpeed(i32, i32),
}

pub struct MotorState {
    pub is_executing: bool,
    pub file_open: bool,
    pub connect_active: bool,
    pub last_response: MotorResponse,

    pub has_homed: bool,
    pub has_calibrated: bool,

    pub size: (u16, u16),
}

impl MotorState {
    pub fn new() -> Self {
        Self {
            is_executing: false,
            file_open: false,
            connect_active: false,
            last_response: MotorResponse::Ok,
            has_homed: false,
            has_calibrated: false,
            size: (0, 0),
        }
    }
}

pub fn motor_thread(commands: Receiver<MotorCommand>, state: Arc<RwLock<MotorState>>) {
    let _ = run_motor_thread(commands, state);
}

pub fn run_motor_thread(
    commands: Receiver<MotorCommand>,
    state: Arc<RwLock<MotorState>>,
) -> Result<(), RecvError> {
    loop {
        let mut motor = match Motor::new("/dev/ttyACM0") {
            Ok(motor) => motor,
            Err(e) => {
                state.write().file_open = false;
                println!("Motor not found: {e}");
                commands.recv()?;
                continue;
            }
        };
        state.write().file_open = true;
        match motor.wait_for_ready() {
            Ok(_) => (),
            Err(_) => continue,
        }

        let mut state_w = state.write();
        state_w.connect_active = true;
        state_w.is_executing = false;
        state_w.has_calibrated = false;
        state_w.has_homed = false;
        drop(state_w);

        loop {
            let command = commands.recv()?;
            state.write().is_executing = true;

            let mut new_size = None;

            let result = match &command {
                MotorCommand::Wakeup => {
                    continue;
                }
                MotorCommand::MoveTo(x, y) => motor.goto(*x as u16, *y as u16),
                MotorCommand::MoveToGrid(x, y) => {
                    let x = motor.goto_grid(*x, *y);
                    println!("Response from move_to_grid");

                    x
                }
                MotorCommand::Home => match motor.home() {
                    Ok((resp, size)) => {
                        new_size = Some(size);
                        Ok(resp)
                    }
                    Err(e) => Err(e),
                },
                MotorCommand::Calibrate(tl, tr, bl) => motor.calibrate_3point(
                    (tl.0 as _, tl.1 as _),
                    (tr.0 as _, tr.1 as _),
                    (bl.0 as _, bl.1 as _),
                ),
                MotorCommand::MotorExecute(cmds) => {
                    let new_commands = convert_commands(cmds);

                    motor.execute_in_order(&new_commands)
                }
                MotorCommand::PenUp => motor.pen_up(),
                MotorCommand::PenDown => motor.pen_down(),
                MotorCommand::SetAutoPenup(auto_penup) => motor.set_auto_pen_up(*auto_penup),
                MotorCommand::SetSpeed(speed, accel) => motor.set_motor_speed(*speed, *accel),
            };

            match result {
                Ok(response) => {
                    let mut state = state.write();
                    state.last_response = response;
                    state.is_executing = false;

                    if response == MotorResponse::Ok {
                        if let Some(new_size) = new_size {
                            state.size = new_size;
                        }
                        match command {
                            MotorCommand::Calibrate(_, _, _) => state.has_calibrated = true,
                            MotorCommand::Home => state.has_homed = true,
                            _ => (),
                        }
                    } else if response == MotorResponse::Reset {
                        state.has_homed = false;
                        state.has_calibrated = false;
                        match motor.send_send_ack() {
                            Ok(_) => (),
                            Err(_) => break,
                        }
                    }
                }
                Err(e) => {
                    state.write().connect_active = false;
                    println!("Motor IO error: {e}");
                    break;
                }
            }
        }
    }
}

pub fn convert_commands(cmds: &Vec<Instruction>) -> Vec<SolvingCommand> {
    let mut current_loc = (0, 0);
    let mut prev_instr = None;

    let mut new_commands = vec![];

    for cmd in cmds {
        if let Some(prev_instr) = &prev_instr {
            if *prev_instr != *cmd {
                if let Instruction::Goto(_, _) = prev_instr {
                    new_commands.push(SolvingCommand::PenUp);
                }
                new_commands.push(SolvingCommand::Goto(current_loc.0, current_loc.1));
                if let Instruction::Goto(_, _) = prev_instr {
                    new_commands.push(SolvingCommand::PenDown);
                }
            }
        }

        match cmd {
            Instruction::Up => current_loc.1 -= 1,
            Instruction::Down => current_loc.1 += 1,
            Instruction::Left => current_loc.0 -= 1,
            Instruction::Right => current_loc.0 += 1,
            Instruction::Goto(x, y) => current_loc = (*x, *y),
            instr => warn!("Not yet implemented: {:?}", instr),
        };

        prev_instr = Some(*cmd);
    }
    new_commands.push(SolvingCommand::Goto(current_loc.0, current_loc.1));
    new_commands.push(SolvingCommand::PenUp);
    new_commands
}
