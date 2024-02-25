use std::{thread, time::Duration};

use serialport::{Error, SerialPort, TTYPort};

#[derive(Debug)]
pub enum SolvingCommand {
    Goto(u8, u8),
    PenUp,
    PenDown,
}

impl SolvingCommand {
    pub fn as_bytes(&self, data: &mut Vec<u8>) {
        match self {
            Self::Goto(x, y) => data.extend_from_slice(&[0, *x, *y]),
            Self::PenUp => data.push(1),
            Self::PenDown => data.push(2),
        }
    }
}

#[repr(u8)]
pub enum MotorCommand {
    Ping = 0,
    BeginHoming = 1,
    GotoAbsolute,
    SetGridCoords,
    GotoGrid,

    InstructionChain,
    PenUp,
    PenDown,
    AutoPenupOn,
    AutoPenupOff,
}

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum MotorResponse {
    Ok = 0,
    Crashed,
    NeedToHome,
    NeedToCalibrate,
    Reset,
    Unknown(u8),
}

impl MotorResponse {
    pub fn ok(self) -> bool {
        self == Self::Ok
    }
}

impl From<u8> for MotorResponse {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Ok,
            1 => Self::Crashed,
            2 => Self::NeedToHome,
            3 => Self::NeedToCalibrate,
            0xFF => Self::Reset,
            x => Self::Unknown(x),
        }
    }
}

pub struct Motor<S> {
    port: S,
}

impl Motor<TTYPort> {
    pub fn new(port: &str) -> Result<Self, Error> {
        let port = serialport::new(port, 9600)
            .timeout(Duration::from_secs(300))
            .open_native()?;
        Ok(Self { port })
    }
}

impl<S> Motor<S>
where
    S: SerialPort,
{
    fn wait_response(&mut self) -> Result<MotorResponse, Error> {
        let mut bytes = [0];
        self.port.read_exact(&mut bytes)?;

        Ok(bytes[0].into())
    }

    pub fn ping(&mut self) -> Result<(), Error> {
        self.port.write_all(&[MotorCommand::Ping as u8])?;
        if self.wait_response()? != MotorResponse::Ok {
            //todo
            panic!("Not ok");
        }
        Ok(())
    }

    pub fn wait_for_ready(&mut self) -> Result<(), Error> {
        let mut bytes = [0];
        while bytes[0] != 0xFF {
            self.port.read_exact(&mut bytes)?;
        }
        self.port.write_all(&[255, 255, 255, 255, 255])?;

        Ok(())
    }

    pub fn send_send_ack(&mut self) -> Result<(), Error> {
        Ok(self
            .port
            .write_all(&[255, 255, 255, 255, 255])
            .map(|_| ())?)
    }

    pub fn goto(&mut self, x: u16, y: u16) -> Result<MotorResponse, Error> {
        println!("Goto {},{}", x, y);
        let xb = x.to_le_bytes();
        let yb = y.to_le_bytes();

        self.port
            .write_all(&[MotorCommand::GotoAbsolute as _, xb[0], xb[1], yb[0], yb[1]])?;
        self.wait_response()
    }

    pub fn goto_grid(&mut self, x: u8, y: u8) -> Result<MotorResponse, Error> {
        self.port.write_all(&[MotorCommand::GotoGrid as _, x, y])?;
        println!("Waiting for response...");
        self.wait_response()
    }

    pub fn calibrate_3point(
        &mut self,
        tl: (u16, u16),
        tr: (u16, u16),
        bl: (u16, u16),
    ) -> Result<MotorResponse, Error> {
        println!("Calibrate: {tl:?}, {tr:?}, {bl:?}");
        let mut points = [0u16; 25 * 2];
        let h = (tr.0 - tl.0, tr.1 - tl.1).0 / 4; // leave out y part
        let v = (bl.0 - tl.0, bl.1 - tl.1).1 / 4; // leave out x part

        for (i, p) in points.chunks_mut(2).enumerate() {
            let x = (i % 5) as u16;
            let y = (i / 5) as u16;

            p[0] = tl.0 + x * h;
            p[1] = tl.1 + y * v;
        }

        println!("{:?}", points);

        let bytes: Vec<u8> = points
            .into_iter()
            .map(|x| x.to_le_bytes())
            .flatten()
            .collect();

        self.port.write_all(&[MotorCommand::SetGridCoords as u8])?;
        self.port.write_all(&bytes)?;

        self.wait_response()
    }

    pub fn home(&mut self) -> Result<(MotorResponse, (u16, u16)), Error> {
        println!("Homing!");
        self.port.write_all(&[MotorCommand::BeginHoming as u8])?;
        self.port.flush()?;

        let resp = self.wait_response()?;

        let mut x = [0; 2];
        let mut y = [0; 2];
        if resp.ok() {
            self.port.read_exact(&mut x)?;
            self.port.read_exact(&mut y)?;
        }

        Ok((resp, (u16::from_le_bytes(x), u16::from_le_bytes(y))))
    }

    pub fn execute_in_order(
        &mut self,
        commands: &[SolvingCommand],
    ) -> Result<MotorResponse, Error> {
        println!("Executing...");
        let mut cmds = Vec::new();
        for cmd in commands.iter() {
            cmd.as_bytes(&mut cmds);
        }

        if cmds.len() > 255 {
            println!("TOO LONG COMMAND CHAIN");
            return Ok(MotorResponse::Ok);
        }

        println!("Instruction count: {}", cmds.len());
        println!("Instructions: {:?}", cmds);

        self.port
            .write_all(&[MotorCommand::InstructionChain as u8, cmds.len() as u8])?;
        thread::sleep(Duration::from_millis(10));
        self.port.write_all(&cmds)?;

        self.wait_response()
    }

    pub fn pen_up(&mut self) -> Result<MotorResponse, Error> {
        self.port.write_all(&[MotorCommand::PenUp as u8])?;
        self.wait_response()
    }

    pub fn pen_down(&mut self) -> Result<MotorResponse, Error> {
        self.port.write_all(&[MotorCommand::PenDown as u8])?;
        self.wait_response()
    }

    pub fn set_auto_pen_up(&mut self, auto_up: bool) -> Result<MotorResponse, Error> {
        if auto_up {
            self.port.write_all(&[MotorCommand::AutoPenupOn as u8])?;
        } else {
            self.port.write_all(&[MotorCommand::AutoPenupOff as u8])?;
        }
        self.wait_response()
    }
}
