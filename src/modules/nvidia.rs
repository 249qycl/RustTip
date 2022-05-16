use super::config;
use regex::Regex;
use std::process::Command;

#[derive(Debug)]
pub struct Nvidia {
    used_memory: u16,
    total_memory: u16,
    use_ratio: u8,
    counter_free: u32,
    counter_efficiency: u32,
}
impl Nvidia {
    pub fn new() -> Nvidia {
        Nvidia {
            used_memory: 0,
            total_memory: 0,
            use_ratio: 0,
            counter_free: 0,
            counter_efficiency: 0,
        }
    }
    pub fn read_from_terminal(&mut self) {
        let output = Command::new("nvidia-smi")
            .output()
            .expect("命令执行异常错误提示");
        let message = String::from_utf8(output.stdout).unwrap();
        let re = Regex::new(r"\d{1,3}%").unwrap();
        let caps = re.captures(&message).unwrap();
        self.use_ratio = caps
            .get(0)
            .map_or("", |m| m.as_str())
            .replace("%", "")
            .parse()
            .unwrap();

        let re = Regex::new(r"\d{1,5}MiB").unwrap();
        let mut caps = re.captures_iter(&message).into_iter();

        self.used_memory = caps
            .next()
            .unwrap()
            .get(0)
            .map_or("", |m| m.as_str())
            .replace("MiB", "")
            .parse()
            .unwrap();
        self.total_memory = caps
            .next()
            .unwrap()
            .get(0)
            .map_or("", |m| m.as_str())
            .replace("MiB", "")
            .parse()
            .unwrap();
    }
    pub fn is_free(&mut self) -> bool {
        if self.used_memory as f32 / (self.total_memory as f32) < 0.10 && self.use_ratio < 5 {
            self.counter_free += 1;
            if self.counter_free > config::DEVICE_FREE {
                self.counter_free = 0;
                return true;
            }
        } else {
            self.counter_free = 0;
        }
        false
    }
    pub fn is_low_efficiency(&mut self) -> bool {
        let mem_ratio = self.used_memory as f32 / self.total_memory as f32;
        if mem_ratio >= 0.10 && mem_ratio <= 0.5 && self.use_ratio >= 5 && self.use_ratio <= 50 {
            self.counter_efficiency += 1;
            if self.counter_efficiency > config::DEVICE_LOW_EFFICIENCY {
                self.counter_efficiency = 0;
                return true;
            }
        } else {
            self.counter_efficiency = 0;
        }
        false
    }
}
