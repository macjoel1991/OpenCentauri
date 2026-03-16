use std::fs;
use std::io;
use std::os::unix::io::AsRawFd;
use std::thread;
use std::time::Duration;

use nix::ioctl_readwrite;

const V4L2_CTRL_CLASS_USER: u32 = 0x00980000;
const V4L2_CID_BASE: u32 = V4L2_CTRL_CLASS_USER | 0x900;
const V4L2_CID_BACKLIGHT_COMPENSATION: u32 = V4L2_CID_BASE + 28;

#[repr(C)]
struct V4l2Control {
    id: u32,
    value: i32,
}

// VIDIOC_S_CTRL = _IOWR('V', 28, struct v4l2_control)
ioctl_readwrite!(vidioc_s_ctrl, b'V', 28, V4l2Control);

const GPIO_PIN: u32 = 207;
const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Open /dev/video0 and set V4L2_CID_BACKLIGHT_COMPENSATION to `light_state`.
fn camera_control_light(light_state: bool) -> io::Result<()> {
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/video0")
        .map_err(|e| {
            eprintln!("open camera failed: {}", e);
            e
        })?;

    let mut control = V4l2Control {
        id: V4L2_CID_BACKLIGHT_COMPENSATION,
        value: light_state as i32,
    };

    unsafe { vidioc_s_ctrl(file.as_raw_fd(), &mut control) }.map_err(|e| {
        let err = io::Error::from(e);
        eprintln!("camera control light error: {}", err);
        err
    })?;

    Ok(())
}

/// Read the current value of a sysfs GPIO pin.
/// Returns `true` for high (1) and `false` for low (0).
fn read_gpio(pin: u32) -> io::Result<bool> {
    let path = format!("/sys/class/gpio/gpio{}/value", pin);
    let raw = fs::read_to_string(&path)?;
    let value: u8 = raw.trim().parse().map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, e)
    })?;
    Ok(value != 0)
}

fn main() -> io::Result<()> {
    let _ = fs::write("/sys/class/gpio/export", GPIO_PIN.to_string());
    let mut last_state = false;

    loop {
        thread::sleep(POLL_INTERVAL);

        let current_state = read_gpio(GPIO_PIN)?;

        if current_state != last_state {
            println!("GPIO state changed: {} -> {}", last_state, current_state);
            last_state = current_state;
            if let Err(e) = camera_control_light(current_state) {
                eprintln!("Failed to control camera light: {}", e);
            }
        }
    }
}
