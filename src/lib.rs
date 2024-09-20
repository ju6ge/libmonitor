//! DDC Driver Crate
//!
//! This create aims to unify multiple singular crates for monitor communcation

pub mod ddc;
pub mod mccs;

use ddc::{edid::Edid, Ddc, DdcError};
use mccs::{
    capabilities::Capabilities,
    features::{queue::VcpCodeUpdateQueue, ContrastValue, InputSource, LuminanceValue, OsdLanguages},
};
use std::{fmt::Display, io};
use thiserror::Error;

#[cfg(target_os = "linux")]
use crate::ddc::linux::{LinuxDdcDevice, LinuxDdcDeviceEnumerator};

/// The error type for high level DDC/CI monitor operations.
#[derive(Debug, Error)]
pub enum DisplayError {
    /// Unsupported operation.
    #[error("the backend does not support the operation")]
    UnsupportedOp,

    /// An error occurred while reading the Edid Data.
    #[error("failed to read edid: {0}")]
    DdcError(#[from] DdcError),

    /// An error io error occured
    #[error("data read failed: {0}")]
    IoError(#[from] io::Error),
}

/// Identifying information about an attached display.
///
/// Not all information will be available, particularly on backends like
/// WinAPI that do not support EDID.
//#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Clone, Debug)]
pub struct MonitorInfo {
    edid: Edid,
    mccs_features: Option<Capabilities>,
}

impl MonitorInfo {
    pub fn manufacture_year(&self) -> usize {
        self.edid.header.year as usize + 1990
    }

    pub fn serial(&self) -> u32 {
        self.edid.header.serial
    }

    pub fn capabilities(&self) -> Option<&Capabilities> {
        self.mccs_features.as_ref()
    }
}

/// An active handle to a connected display.
pub struct MonitorDevice<D>
where
    D: Ddc,
{
    /// The inner communication handle used for DDC commands.
    pub handle: Box<D>,
    /// Information about the connected display.
    pub info: MonitorInfo,
}

impl<D> Display for MonitorDevice<D>
where
    D: Ddc,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("Monitor ({})", self.handle.name()))
            .field("serial", &self.info.serial())
            .field("manufacture_year", &self.info.manufacture_year())
            .finish()
    }
}

impl<D> MonitorDevice<D>
where
    D: Ddc,
{
    /// Create a new display from the specified handle.
    pub fn new(mut handle: D) -> Result<Self, DisplayError> {
        let edid = handle.read_edid()?;
        Ok(MonitorDevice {
            handle: Box::new(handle),
            info: MonitorInfo {
                edid,
                mccs_features: None,
            },
        })
    }

    pub fn event_iter(&mut self) -> VcpCodeUpdateQueue<D> {
       VcpCodeUpdateQueue::new(&mut self.handle)
    }

    /// get the currently active monitor input source
    pub fn get_input_source(&mut self) -> Result<InputSource, DdcError> {
        self.handle.get_vcp_feature()
    }

    /// set the currently active monitor input
    pub fn set_input_source(&mut self, input_source: InputSource) -> Result<(), DdcError> {
        self.handle.set_vcp_feature(input_source)
    }

    /// get the currently selected monitor on screen display language
    pub fn get_language(&mut self) -> Result<OsdLanguages, DdcError> {
        self.handle.get_vcp_feature()
    }

    /// set the monitor on screen language
    pub fn set_language(&mut self, language: OsdLanguages) -> Result<(), DdcError> {
        self.handle.set_vcp_feature(language)
    }

    /// read the current monitor brightness and map it to a value between 0 and 1
    pub fn get_luminance(&mut self) -> Result<f64, DdcError> {
        let luminance: LuminanceValue = self.handle.get_vcp_feature()?;
        Ok((luminance.val as f64) / luminance.max as f64)
    }

    /// set the current monitor brightness, supplied value should be in range 0 <= val <= 1
    pub fn set_luminance(&mut self, lum: f64) -> Result<(), DdcError> {
        assert!(lum >= 0. && lum <= 1.);
        let mut luminance: LuminanceValue = self.handle.get_vcp_feature()?;
        luminance.val = ((luminance.max as f64) * lum).round() as u16;
        self.handle.set_vcp_feature(luminance)
    }

    /// read the current monitor contrast and map it to a value between 0 and 1
    pub fn get_contrast(&mut self) -> Result<f64, DdcError> {
        let contrast: ContrastValue = self.handle.get_vcp_feature()?;
        Ok((contrast.val as f64) / contrast.max as f64)
    }

    /// set the current monitor contrast, supplied value should be in range 0 <= val <= 1
    pub fn set_contrast(&mut self, lum: f64) -> Result<(), DdcError> {
        assert!(lum >= 0. && lum <= 1.);
        let mut contrast: ContrastValue = self.handle.get_vcp_feature()?;
        contrast.val = ((contrast.max as f64) * lum).round() as u16;
        self.handle.set_vcp_feature(contrast)
    }
}

#[cfg(target_os = "linux")]
pub type Monitor = MonitorDevice<LinuxDdcDevice>;

impl Monitor {
    #[cfg(target_os = "linux")]
    /// Enumerate all currently attached monitor devices
    ///
    /// ```rust
    /// use libmonitor::Monitor;
    ///
    /// for monitor in Monitor::enumerate() {
    ///     println!("{monitor:#}")
    /// }
    /// ```
    pub fn enumerate() -> MonitorIterator<LinuxDdcDevice> {
        MonitorIterator {
            inner_iter: Box::new(LinuxDdcDeviceEnumerator::iter()),
        }
    }
}

pub struct MonitorIterator<D>
where
    D: Ddc,
{
    inner_iter: Box<dyn Iterator<Item = D>>,
}

impl<D> Iterator for MonitorIterator<D>
where
    D: Ddc,
{
    type Item = MonitorDevice<D>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner_iter
            .next()
            .and_then(|dev| MonitorDevice::new(dev).ok())
    }
}
