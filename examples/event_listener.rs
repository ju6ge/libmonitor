use std::{thread::sleep, time::Duration};

use i2cdev::{
    core::I2CTransfer,
    linux::{I2CMessage, LinuxI2CBus},
};
use libmonitor::{ddc::DdcCiError, Monitor};
use libmonitor::{
    ddc::{
        linux::receive_edid, Ddc, DdcCiDevice, DdcCommunicationBase, DdcDevice, DdcError,
        DeriveDdcCiDevice, I2C_DDC_RECV_BUFFER_SIZE,
    },
    MonitorDevice,
};

struct MyDccI2CBus(LinuxI2CBus);

impl DdcCommunicationBase for MyDccI2CBus {
    fn transmit(&mut self, addr: u8, data: &[u8]) -> Result<(), DdcCiError> {
        let msg = i2cdev::linux::LinuxI2CMessage::write(data).with_address(addr.into());
        self.0
            .transfer(&mut vec![msg])
            .map_err(|err| DdcCiError::TransmitError(anyhow::Error::new(err)))?;
        Ok(())
    }
    /// implement raw i2c reading on your device buffer size is limited to 64 bit which is double of the allowed data fragment size for ddc communication
    fn receive(&mut self, addr: u8) -> Result<[u8; I2C_DDC_RECV_BUFFER_SIZE], DdcCiError> {
        let mut data = [0; I2C_DDC_RECV_BUFFER_SIZE];
        data[0] = addr << 1 | 0x1;
        let msg = i2cdev::linux::LinuxI2CMessage::read(&mut data[1..]).with_address(addr.into());
        self.0
            .transfer(&mut [msg])
            .map_err(|err| DdcCiError::ReceiveError(anyhow::Error::new(err)))?;
        Ok(data)
    }
    /// implement delay for your device
    fn delay(&self, delay_ms: u64) {
        std::thread::sleep(Duration::from_millis(delay_ms))
    }
}

impl DeriveDdcCiDevice for MyDccI2CBus {}

impl DdcDevice for MyDccI2CBus {
    fn name(&self) -> String {
        "my_monitor".to_string()
    }

    fn read_edid(&mut self) -> Result<libmonitor::ddc::edid::Edid, libmonitor::ddc::DdcError> {
        receive_edid(&mut self.0)
            .map_err(|err| DdcError::CommunicationError(DdcCiError::ReceiveError(err)))
    }
}

impl Ddc for MyDccI2CBus {}

fn main() {
    let i2cdev = LinuxI2CBus::new("/dev/i2c-22").unwrap();
    let mut monitor = MonitorDevice::new(MyDccI2CBus(i2cdev)).unwrap();
    println!("{monitor:#}");
    //println!("{:#?}", monitor.handle.read_capabilities());
    loop {
        for vcp_event in monitor.event_iter() {
            println!("{vcp_event:#?}");
        }
    }
}
