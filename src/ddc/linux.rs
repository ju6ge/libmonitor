use i2cdev::{
    core::I2CTransfer,
    linux::{I2CMessage, LinuxI2CBus},
};
use std::{ffi::OsStr, fs::File, io::Read, path::Path, time::Duration};
use udev::Device;

use super::{
    eddc::{EDDC_SEGMENT_POINTER_ADDR, EDID_ADDRESS},
    edid::{parse_edid, Edid},
    Ddc, DdcCiError, DdcCommunicationBase, DdcDevice, DeriveDdcCiDevice,
};

const RECEIVE_EDID_RETRIES: u8 = 3;

/// this function only reads the first 128 of edid, this
/// can be reasonably assumed to be present on all display devices
pub fn receive_edid(i2c_bus: &mut LinuxI2CBus) -> Result<Edid, anyhow::Error> {
    // reset eddc segment pointer. May fail if display does not implement eddc for specific input
    // some displays behave differently depending on the input source.
    let _ = i2c_bus.transfer(&mut [i2cdev::linux::LinuxI2CMessage::write(&[0x0])
        .with_address(EDDC_SEGMENT_POINTER_ADDR.into())]);

    let mut receive_try = RECEIVE_EDID_RETRIES;
    loop {
        //initiate edid reading
        i2c_bus
            .transfer(&mut [
                i2cdev::linux::LinuxI2CMessage::write(&[0x0]).with_address(EDID_ADDRESS.into())
            ])
            .map_err(|err| anyhow::Error::new(err))?;
        //read first 128 bytes of edid
        let mut data: [u8; 128] = [0; 128];
        let _ = i2c_bus
            .transfer(&mut [
                i2cdev::linux::LinuxI2CMessage::read(&mut data).with_address(EDID_ADDRESS.into())
            ])
            .map_err(|err| anyhow::Error::new(err))?;
        let x = parse_edid(&data).map_err(|err| anyhow::Error::new(err));
        if receive_try == 0 {
            return x;
        } else {
            receive_try -= 1;
            if x.is_ok() {
                return x;
            }
        }
    }
}

// filter phantom devices, devices connected via docking stations may appear as two seperate
// i2c devices with only one working (workarount copied from ddcutil)
fn is_phantom_ddc_device(id: usize) -> bool {
    use std::{fs::File, io::Read};

    let device_path_prefix = Path::new("/sys/bus/i2c/devices");
    let device_path = device_path_prefix.join(format!("i2c-{id}"));
    if device_path.exists() {
        let enabled_path = device_path.join("device").join("enabled");
        let status_path = device_path.join("device").join("status");
        if enabled_path.exists()
            && File::open(enabled_path)
                .and_then(|mut f| {
                    let mut content = String::new();
                    f.read_to_string(&mut content)?;
                    Ok(content.trim().to_string())
                })
                .is_ok_and(|content| content == "disabled")
            && status_path.exists()
            && File::open(status_path)
                .and_then(|mut f| {
                    let mut content = String::new();
                    f.read_to_string(&mut content)?;
                    Ok(content.trim().to_string())
                })
                .is_ok_and(|content| content == "disconnected")
        {
            true
        } else {
            false
        }
    } else {
        // it can not be a valid device if it is not found in the
        // system tree
        true
    }
}

// ignore devices that are probably not related to a monitor
fn ignore_device_by_name(name: &OsStr) -> bool {
    // list stolen from ddcutil's ignorable_i2c_device_sysfs_name
    let skip_prefix = ["SMBus", "soc:i2cdsi", "smu", "mac-io", "u4"];

    name.to_str().is_some_and(|name| {
        for prefix in skip_prefix {
            if name.starts_with(prefix) {
                return true;
            }
        }
        false
    })
}

// check if the parent device is a graphics device
fn device_is_display(dev: &udev::Device) -> bool {
    dev.parent().is_some_and(|i2c_parent| {
        i2c_parent.parent().is_some_and(|maybe_graphics_device| {
            maybe_graphics_device
                .subsystem()
                .is_some_and(|subsystem| subsystem == "drm")
                || maybe_graphics_device
                    .property_value("ID_PCI_CLASS_FROM_DATABASE")
                    .is_some_and(|class| class == "Display controller")
        })
    })
}

fn find_parent_drm_device(i2c_dev: &udev::Device) -> Option<Device> {
    if let Some(i2c_parent) = i2c_dev.parent() {
        // assuming this is a graphics device, other devices should have been filtered beforehand
        if let Some(graphics_device) = i2c_parent.parent() {
            if graphics_device
                .subsystem()
                .is_some_and(|subsystem| subsystem == "drm")
            {
                // display device with type drm are easy because it is already correctly mapped
                return Some(graphics_device);
            } else if graphics_device
                .property_value("ID_PCI_CLASS_FROM_DATABASE")
                .is_some_and(|class| class == "Display controller")
            {
                //display i2c bus but not of type drm are harder because no we need to loop over all devices of class drm and
                //find one with a matiching edid data to the one read via the i2c channel

                let mut i2c =
                    LinuxI2CBus::new(format!("/dev/i2c-{}", i2c_dev.sysnum().unwrap())).unwrap();
                match receive_edid(&mut i2c) {
                    Ok(i2c_edid) => {
                        let mut drm_enum = udev::Enumerator::new().unwrap();
                        drm_enum.match_subsystem("drm").ok();
                        let devices = drm_enum.scan_devices().unwrap();

                        for (drm_device, edid_data) in devices.filter_map(|dev| {
                            // only consider drm devices
                            let edid_path = dev.syspath().join("edid");
                            let mut edid_data = [0 as u8; 128];
                            if edid_path.exists()
                                && File::open(&edid_path)
                                    .unwrap()
                                    .read(&mut edid_data)
                                    .is_ok_and(|size| size > 0)
                            {
                                Some((dev, edid_data))
                            } else {
                                None
                            }
                        }) {
                            if parse_edid(&edid_data).is_ok_and(|drm_edid| drm_edid == i2c_edid) {
                                return Some(drm_device);
                            }
                        }
                    }
                    Err(_err) => { /*println!("{err:#?}");*/ }
                };
            }
        }
    }
    None
}

pub struct LinuxDdcDevice {
    i2c_sysnum: usize,
    drm_device: udev::Device,
}

impl LinuxDdcDevice {
    pub fn new(i2c_sysnum: usize, drm_device: udev::Device) -> Self {
        Self {
            i2c_sysnum,
            drm_device,
        }
    }

    fn device_sysnum(&self) -> usize {
        self.i2c_sysnum
    }

    fn open_i2c_bus(&self) -> LinuxI2CBus {
        LinuxI2CBus::new(format!("/dev/i2c-{}", self.device_sysnum())).unwrap()
    }
}

impl DdcCommunicationBase for LinuxDdcDevice {
    fn delay(&self, delay_ms: u64) {
        std::thread::sleep(Duration::from_millis(delay_ms))
    }

    fn transmit(&mut self, addr: u8, data: &[u8]) -> Result<(), super::DdcCiError> {
        let msg = i2cdev::linux::LinuxI2CMessage::write(data).with_address(addr.into());
        self.open_i2c_bus()
            .transfer(&mut [msg])
            .map_err(|err| DdcCiError::TransmitError(anyhow::Error::new(err)))?;
        Ok(())
    }

    fn receive(
        &mut self,
        addr: u8,
    ) -> Result<[u8; super::I2C_DDC_RECV_BUFFER_SIZE], super::DdcCiError> {
        let mut data = [0; super::I2C_DDC_RECV_BUFFER_SIZE];
        data[0] = addr << 1 | 0x1;
        let msg = i2cdev::linux::LinuxI2CMessage::read(&mut data[1..]).with_address(addr.into());
        self.open_i2c_bus()
            .transfer(&mut [msg])
            .map_err(|err| DdcCiError::ReceiveError(anyhow::Error::new(err)))?;
        Ok(data)
    }
}

impl DdcDevice for LinuxDdcDevice {
    fn name(&self) -> String {
        self.drm_device
            .sysname()
            .to_str()
            .unwrap()
            .split_once('-')
            .unwrap()
            .1
            .to_string()
    }

    fn read_edid(&mut self) -> Result<super::edid::Edid, super::DdcError> {
        let edid_path = self.drm_device.syspath().join("edid");
        let mut edid_data = File::open(edid_path)?;
        let mut data = [0 as u8; 128];
        let _size = edid_data.read(&mut data)?;
        Ok(parse_edid(&data)?)
    }
}

impl DeriveDdcCiDevice for LinuxDdcDevice {}
impl Ddc for LinuxDdcDevice {}

struct LinuxDrmI2C {
    drm_device: udev::Device,
    i2c_device: udev::Device,
}

pub struct LinuxDdcDeviceEnumerator {
    inner_iter: Box<dyn Iterator<Item = LinuxDrmI2C>>,
}

impl LinuxDdcDeviceEnumerator {
    pub fn iter() -> Self {
        let mut i2c_enum = udev::Enumerator::new().unwrap();
        i2c_enum.match_subsystem("i2c-dev").ok();

        let devices = i2c_enum.scan_devices().unwrap();
        let filtered: Vec<LinuxDrmI2C> = devices
            .into_iter()
            .filter(|dev| {
                dev.attribute_value("name")
                    .is_some_and(|name| !ignore_device_by_name(name))
            })
            .filter(|dev| device_is_display(dev))
            .filter(|dev| dev.sysnum().is_some_and(|id| !is_phantom_ddc_device(id)))
            .filter_map(|i2c_device| {
                if let Some(drm_device) = find_parent_drm_device(&i2c_device) {
                    Some(LinuxDrmI2C {
                        drm_device,
                        i2c_device,
                    })
                } else {
                    None
                }
            })
            .collect();
        Self {
            inner_iter: Box::new(filtered.into_iter()),
        }
    }
}

impl Iterator for LinuxDdcDeviceEnumerator {
    type Item = LinuxDdcDevice;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner_iter.next().and_then(|dev| {
            dev.i2c_device
                .sysnum()
                .map(|id| LinuxDdcDevice::new(id, dev.drm_device))
        })
    }
}
