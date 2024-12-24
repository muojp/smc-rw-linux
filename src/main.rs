#[link(name = "smc_rw")]
extern "C" {
    fn port_outb(value: u8, port: u16);
    fn port_inb(port: u16) -> u8;
    fn port_ioperm(from: libc::c_ulong, num: libc::c_ulong, turn_on: libc::c_int) -> libc::c_int;
}

const SMC_MIN_WAIT: u32 = 0x10;
const SMC_RETRY_WAIT: u32 = 0x100;
const SMC_MAX_WAIT: u32 = 0x20000;

const SMC_START: u16 = 0x300;
const SMC_RANGE: u8 = 0x20;
const SMC_CMD_PORT: u16 = SMC_START + 0x04;
const SMC_DATA_PORT: u16 = SMC_START + 0x00;

const SMC_KEY_NAME_LEN: usize = 4;
const SMC_KEY_TYPE_LEN: usize = 4;

const SMC_GET_KEY_TYPE_CMD: u8 = 0x13;
const SMC_READ_CMD: u8 = 0x10;
const SMC_WRITE_CMD: u8 = 0x11;

const CMD_READ_ONLY: i32 = -1;

use std::mem;
use std::process;
use clap::Parser;
use std::str::FromStr;

#[cfg(test)]
use mockall::{automock, predicate::*};

#[cfg(test)]
mod main_test;

#[repr(C, packed)]
struct SmcData {
    data_len: u8,
    data_type: [u8; SMC_KEY_TYPE_LEN],
    flags: u8
}

#[cfg_attr(test, automock)]
trait IoPortRw {
    fn delayed_outb(&self, us: u32, value: u8, port: u16);
    fn delayed_inb(&self, us: u32, port: u16) -> u8;
    fn ioperm (&self, from: libc::c_ulong, num: libc::c_ulong, turn_on: libc::c_int) -> libc::c_int;
}

struct LinuxIoPortRw {}

impl IoPortRw for LinuxIoPortRw {
    fn delayed_outb(&self, us: u32, value: u8, port: u16) {
        unsafe {
            if us != 0 {
                libc::usleep(us);
            }
            port_outb(value, port)
        }
    }

    fn delayed_inb(&self, us: u32, port: u16) -> u8 {
        unsafe {
            if us != 0 {
                libc::usleep(us);
            }
            port_inb(port)
        }
    }

    fn ioperm (&self, from: libc::c_ulong, num: libc::c_ulong, turn_on: libc::c_int) -> libc::c_int {
        unsafe {
            port_ioperm(from, num, turn_on)
        }
    }
}

#[cfg_attr(test, automock(type IoPort=MockIoPortRw;))]
trait SmcPrimitive {
    type IoPort: IoPortRw;
    fn new(io_port_rw: Self::IoPort) -> Self;
    fn wait_read(&self) -> libc::c_int;
    fn send_byte(&self, cmd: u8, port: u16) -> Result<libc::c_int, String>;
    fn recv_byte(&self) -> Result<u8, ()>;
    fn send_argument(&self, key: [u8; SMC_KEY_NAME_LEN]) -> Result<(), (usize, String)>;
}

trait SmcOperation {
    fn read_smc(&self, cmd: u8, key: [u8; SMC_KEY_NAME_LEN], buf: &mut [u8]) -> Result<libc::c_int, String> where Self: SmcPrimitive {
        if self.send_byte(cmd, SMC_CMD_PORT).is_err() || self.send_argument(key).is_err() {
            return Err(format!("{:?}: read arg failed", key));
        }
        if self.send_byte(buf.len().try_into().map_err(|_| "data len limit exceeded")?, SMC_DATA_PORT).is_err() {
            return Err(format!("{:?}: read len failed", key));
        }

        for i in 0..buf.len() {
            if self.wait_read() == -1 {
                return Err(format!("{:?}: read data {} failed", key, i));
            }
            buf[i] = self.recv_byte().unwrap();
        }

        Ok(0)
    }

    fn write_smc(&self, cmd: u8, key: [u8; SMC_KEY_NAME_LEN], buf: &[u8]) -> Result<libc::c_int, String> where Self: SmcPrimitive {
        if self.send_byte(cmd, SMC_CMD_PORT).is_err() || self.send_argument(key).is_err() {
            return Err(format!("{:?}: write arg failed", key));
        }

        if self.send_byte(buf.len().try_into().map_err(|_| "data len limit exceeded")?, SMC_DATA_PORT).is_err() {
            return Err(format!("{:?}: write len failed", key))
        }

        for i in 0..buf.len() {
            self.send_byte(buf[i], SMC_DATA_PORT)?;
        }

        Ok(0)
    }

    fn read(&self, reg: [u8; SMC_KEY_NAME_LEN]) -> Result<(), String> where Self: SmcPrimitive {
        let mut smc_data = SmcData { data_len: 0, data_type: [0u8; SMC_KEY_TYPE_LEN], flags: 0};
        let buf = unsafe { std::slice::from_raw_parts_mut(&mut smc_data as *mut SmcData as *mut u8, mem::size_of::<SmcData>()) };
        let mut data_buf = [0u8; 255];
    
        self.read_smc(SMC_GET_KEY_TYPE_CMD, reg, buf)?;
        // eprintln!("OK: {:?}, len={}", smc_data.data_type, smc_data.data_len);
    
        self.read_smc(SMC_READ_CMD, reg, &mut data_buf[0..smc_data.data_len as usize])?;
        if smc_data.data_len == 1 {
            eprintln!("{}", &data_buf[0]);
        }
        else {
            eprintln!("{:?}", &data_buf[0..smc_data.data_len as usize]);
        }

        Ok(())
    }

    fn write(&self, reg: [u8; SMC_KEY_NAME_LEN], val: i32) -> Result<(), String> where Self: SmcPrimitive {
        let buf: [u8; 1] = [val.try_into().map_err(|_| "invalid arg".to_string())?];
        self.write_smc(SMC_WRITE_CMD, reg, &buf)?;

        Ok(())
    }
}

struct DefaultSmcRw<T: IoPortRw> {
    io_port_rw: T
}

impl<T: IoPortRw> SmcPrimitive for DefaultSmcRw<T> {
    type IoPort = T;

    fn new(io_port_rw: T) -> Self {
        DefaultSmcRw { io_port_rw }
    }

    fn wait_read(&self) -> libc::c_int {
        let mut us: u32 = SMC_MIN_WAIT;
    
        while us < SMC_MAX_WAIT {
            match self.io_port_rw.delayed_inb(us, SMC_CMD_PORT) & 0x01 {
                0x01 => return 0,
                _ => us <<= 1
            }
        }
    
        -1
    }

    fn send_byte(&self, cmd: u8, port: u16) -> Result<libc::c_int, String> {
        let mut status = 0u8;
        let mut us: u32 = SMC_MIN_WAIT;
        self.io_port_rw.delayed_outb(0, cmd, port);
        while us < SMC_MAX_WAIT {
            status = self.io_port_rw.delayed_inb(us, SMC_CMD_PORT);
            match status & 0x06 {
                0x02 => (),
                0x04 => return Ok(0),
                _ if us << 1 >= SMC_MAX_WAIT => break,
                _ => {
                    self.io_port_rw.delayed_outb(SMC_RETRY_WAIT, cmd, port);
                    us <<= 1;
                }
            }
        }
    
        Err(format!("send_byte(0x{:x}, 0x{:x}) fail: 0x{:x}", cmd, port, status))
    }

    fn recv_byte(&self) -> Result<u8, ()> {
        Ok(self.io_port_rw.delayed_inb(0, SMC_DATA_PORT))
    }

    fn send_argument(&self, key: [u8; SMC_KEY_NAME_LEN]) -> Result<(), (usize, String)> {
        key.iter()
            .enumerate()
            .try_for_each(|(idx, &byte)| {
                self.send_byte(byte, SMC_DATA_PORT)
                    .map(|_| ())
                    .map_err(|s| (idx, s))
            })
    }
}

impl<T: IoPortRw> SmcOperation for DefaultSmcRw<T> {}

#[derive(Debug, Clone)]
struct Code([u8; SMC_KEY_NAME_LEN]);

impl FromStr for Code {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 4 {
            return Err("invalid code length".to_string());
        }

        let bytes: Vec<u8> = s.bytes().collect();
        let array: [u8; SMC_KEY_NAME_LEN] = bytes.try_into()
            .map_err(|_| "data conversion failed".to_string())?;

        Ok(Code(array))
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(value_parser = Code::from_str)]
    code: Code,

    #[arg(value_parser = clap::value_parser!(i32).range(-1..=100), default_value_t = CMD_READ_ONLY)]
    val: i32,
}

fn main() {
    let args = Args::parse();

    let rw = LinuxIoPortRw {};
    if rw.ioperm(SMC_START as u64, SMC_RANGE as u64, 1) != 0 {
        eprintln!("ioperm failed");
        process::exit(1);
    }
    let smc_rw = DefaultSmcRw::new(rw);

    match args.val {
        -1 => {
            if smc_rw.read(args.code.0).is_err() {
                eprintln!("data read failed");
                process::exit(1);
            }
        },
        0..100 => {
            match smc_rw.write(args.code.0, args.val) {
                Err(s) => {
                    eprintln!("data write failed: {}", s);
                    process::exit(1);
                }
                Ok(_) => (),
            }    
        },
        _ => (),
    }
}
