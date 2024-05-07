use libmonitor::Monitor;

pub fn main() {
    for monitor in Monitor::enumerate() {
        println!("{monitor:#}")
    }
}
