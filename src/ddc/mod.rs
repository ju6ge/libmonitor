//! Control displays using the DDC/CI protocol.
//!
//! Provides generic traits and utilities for working with DDC.

/// DDC/CI command messages.
pub mod ci;

/// edid data parsing
pub mod edid;

/// eddc definitons
pub mod eddc;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
mod mac_os;
#[cfg(target_os = "windows")]
mod windows;
use thiserror::Error;

use self::{
    ci::{parse_feature_reply, DdcCiMessage, DdcCiProtocolError, DdcOpcode, ResultCode},
    edid::{Edid, EdidParseError},
};
use crate::mccs::{
    capabilities::{parse_capabilities, Capabilities},
    features::VcpValue,
};

pub const I2C_DDC_RECV_BUFFER_SIZE: usize = 64;

#[derive(Error, Debug)]
pub enum DdcError {
    #[error("Error Reading Data {0}")]
    ReadDataError(#[from] std::io::Error),
    #[error("Error Parsing Edid Data {0}")]
    EdidParseError(#[from] EdidParseError),
    #[error("Internal Displays do not support DDC/CI")]
    InternalDisplay,
    #[error("Communication Error")]
    CommunicationError(#[from] DdcCiError),
    #[error("Unsupported Vcp Feature")]
    UnsupportedVcpFeature,
}

#[derive(Debug, Error)]
pub enum DdcCiError {
    #[error("Error sending DDC data: {0}")]
    TransmitError(anyhow::Error),
    #[error("Error receiving DDC data: {0}")]
    ReceiveError(anyhow::Error),
    #[error("DDC/CI Protocol Error! {0}")]
    ProtocolError(#[from] DdcCiProtocolError),
    #[error("DDC/CI unexpected ReplyCode")]
    UnexpectedReplyCode,
}

/// implement this trait to enable usage of auto implemented ddc functions for you device
pub trait DdcCommunicationBase {
    /// implement raw i2c writing on your device
    fn transmit(&mut self, addr: u8, data: &[u8]) -> Result<(), DdcCiError>;
    /// implement raw i2c reading on your device buffer size is limited to 64 bit which is double of the allowed data fragment size for ddc communication
    fn receive(&mut self, addr: u8) -> Result<[u8; I2C_DDC_RECV_BUFFER_SIZE], DdcCiError>;
    /// implement delay for your device
    fn delay(&self, delay_ms: u64);
}

pub trait DeriveDdcCiDevice: DdcCommunicationBase {}

pub trait DdcCiDevice {
    /// Read Device Capabilities
    fn read_capabilities(&mut self) -> Result<Capabilities, DdcError>;

    /// Gets the current value of an MCCS VCP feature.
    fn get_vcp_feature<V: VcpValue>(&mut self) -> Result<V, DdcError>;

    /// Sets a VCP feature to the specified value.
    fn set_vcp_feature<V: VcpValue>(&mut self, vcp_value: V) -> Result<(), DdcError>;

    /// Instruct the device to save its current settings.
    fn save_current_settings(&mut self) -> Result<(), DdcError>;

    // Retrieves a timing report from the device.
    //fn get_timing_report(&mut self) -> Result<TimingMessage, DdcError> {
    //    todo!()
    //}

    // Read a table value from the device.
    //fn table_read(&mut self, code: VcpFeatureCode) -> Result<Vec<u8>, DdcError>;

    // Write a table value to the device.
    //fn table_write(
    //&mut self,
    //code: VcpFeatureCode,
    //offset: u16,
    //value: &[u8],
    //) -> Result<(), DdcError>;
}

impl<X> DdcCiDevice for X
where
    X: DeriveDdcCiDevice,
{
    fn read_capabilities(&mut self) -> Result<Capabilities, DdcError> {
        let mut capabilities_request =
            DdcCiMessage::from_opcode(ci::DdcOpcode::CapabilitiesRequest).set_offset(0x0);

        // preform initial capabilities request
        self.transmit(
            capabilities_request.addr(),
            &capabilities_request.transmit_buffer(),
        )?;
        self.delay(50);

        // get first capabilities reply
        let mut capabilities_reply =
            DdcCiMessage::parse_buffer(&self.receive(capabilities_request.addr())?)
                .map_err(|err| DdcCiError::ProtocolError(err))?;

        // keep requesting more capabilities data until it has been read compleatly (indicated by a 0 length capabilities reply)
        let mut capabilities_buffer = Vec::new();
        while capabilities_reply.get_data_len() != 0 {
            capabilities_buffer.extend_from_slice(capabilities_reply.get_data());
            // next read should happen from offest + received data length
            capabilities_request =
                capabilities_request.add_offset(capabilities_reply.get_data_len());
            self.transmit(
                capabilities_request.addr(),
                &capabilities_request.transmit_buffer(),
            )?;
            self.delay(50);
            capabilities_reply =
                DdcCiMessage::parse_buffer(&self.receive(capabilities_request.addr())?)
                    .map_err(|err| DdcCiError::ProtocolError(err))?;
        }

        let cap_str = String::from_utf8(capabilities_buffer).unwrap();
        let capabilities: Capabilities = parse_capabilities(&cap_str)?;
        Ok(capabilities)
    }

    fn get_vcp_feature<V: VcpValue>(&mut self) -> Result<V, DdcError> {
        let get_vcp_request =
            DdcCiMessage::from_opcode(ci::DdcOpcode::VcpRequest).set_vcp_feature(V::vcp_feature());
        self.transmit(get_vcp_request.addr(), &get_vcp_request.transmit_buffer())?;
        self.delay(40);
        let mut get_vcp_reply = DdcCiMessage::parse_buffer(&self.receive(get_vcp_request.addr())?)
            .map_err(|err| DdcCiError::ProtocolError(err))?;

        let mut retry = 3;
        // if null message we need to retry after a timout
        while retry > 0 && get_vcp_reply == DdcCiMessage::NullResponse() {
            self.transmit(get_vcp_request.addr(), &get_vcp_request.transmit_buffer())?;
            self.delay(40);
            get_vcp_reply = DdcCiMessage::parse_buffer(&self.receive(get_vcp_request.addr())?)
                .map_err(|err| DdcCiError::ProtocolError(err))?;
            retry -= 1;
        }
        if get_vcp_reply
            .get_opcode()
            .is_some_and(|opcode| *opcode == DdcOpcode::VcpReply)
        {
            let (_, vcp_resp) = parse_feature_reply(get_vcp_reply.get_data()).map_err(|err| {
                println!("{get_vcp_reply:#x?}");
                DdcCiError::ProtocolError(err.into())
            })?;
            if *vcp_resp.result_code() == ResultCode::UnsupportedCode {
                Err(DdcError::UnsupportedVcpFeature)
            } else {
                Ok(vcp_resp.vcp_data().into())
            }
        } else {
            Err(DdcCiError::UnexpectedReplyCode.into())
        }
    }

    fn set_vcp_feature<V: VcpValue>(&mut self, vcp_value: V) -> Result<(), DdcError> {
        let set_vcp_request = DdcCiMessage::from_opcode(ci::DdcOpcode::SetVcp)
            .set_vcp_feature(V::vcp_feature())
            .set_data(&[vcp_value.vh(), vcp_value.vl()])
            .map_err(|err| DdcCiError::ProtocolError(err))?;
        self.transmit(set_vcp_request.addr(), &set_vcp_request.transmit_buffer())?;
        self.delay(50);
        Ok(())
    }

    fn save_current_settings(&mut self) -> Result<(), DdcError> {
        let save_request = DdcCiMessage::from_opcode(ci::DdcOpcode::SaveCurrentSettings);
        self.transmit(save_request.addr(), &save_request.transmit_buffer())?;
        Ok(())
    }
}

pub trait DdcDevice {
    fn name(&self) -> String;

    /// Read Edid Data from Ddc Device
    fn read_edid(&mut self) -> Result<Edid, DdcError>;
}

pub trait Ddc: DdcDevice + DdcCiDevice {}
