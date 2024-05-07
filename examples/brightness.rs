/// Example setting all monitors to maximum brighness

use libmonitor::Monitor;

fn main() {
    for mut monitor in Monitor::enumerate() {
        let _ = monitor.set_luminance(1.0);
    }
}
