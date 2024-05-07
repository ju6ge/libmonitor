use nom::{combinator::peek, number::complete::le_u8};
use nom::{IResult, Parser};
use thiserror::Error;

use crate::mccs::features::VcpFeatureCode;

#[derive(Debug, Error)]
pub enum DdcCiProtocolError {
    #[error("invalid packet lengh")]
    InvalidLength,
    #[error("checksum invalid")]
    InvalidChecksum,
    #[error("Error parsed data does not contain a length field and does not match any other known ddc message format!")]
    InvalidMessageFormat,
    #[error("Error parsing DDC CI Message: {0}")]
    ParserError(String),
}

const DDC_SLAVE_SEND_ADDR: u8 = 0x6f;
const DDC_SLAVE_RECV_ADDR: u8 = 0x6e;
const DDC_MASTER_SEND_ADDR: u8 = 0x51;
const DDC_MASTER_RECV_ADDR: u8 = 0x50;

const LENGTH_PREFIX: u8 = 0x80;

pub const DDC_MAX_DATA_FRAGMENT_LENGTH: usize = 32;
// when receiving unknown opcodes they may or maynot have additional offset or vcp information that
// can not be detected while parsing, so the buffer size here is a bit bigger to allow to capture that data
pub const DDC_MAX_DATA_FRAGMENT_LENGTH_WITH_EXTRA: usize = DDC_MAX_DATA_FRAGMENT_LENGTH + 4;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DdcOpcode {
    IdentificationRequest,
    IdentificationReply,
    CapabilitiesRequest,
    CapabilitiesReply,
    DisplaySelfTestRequest,
    DisplaySelfTestReply,
    TimingRequest,
    TimingReply,
    VcpRequest,
    VcpReply,
    SetVcp,
    ResetVcp,
    TableReadRequest,
    TableReadReply,
    TableWrite,
    EnableApplicationReport,
    SaveCurrentSettings,
    Unknown(u8),
}

impl From<&DdcOpcode> for u8 {
    fn from(value: &DdcOpcode) -> Self {
        match value {
            DdcOpcode::Unknown(value) => *value,
            DdcOpcode::IdentificationRequest => 0xf1,
            DdcOpcode::IdentificationReply => 0xe1,
            DdcOpcode::CapabilitiesRequest => 0xf3,
            DdcOpcode::CapabilitiesReply => 0xe3,
            DdcOpcode::DisplaySelfTestRequest => 0xb1,
            DdcOpcode::DisplaySelfTestReply => 0xa1,
            DdcOpcode::TimingRequest => 0x07,
            DdcOpcode::TimingReply => 0x06,
            DdcOpcode::VcpRequest => 0x01,
            DdcOpcode::VcpReply => 0x02,
            DdcOpcode::SetVcp => 0x03,
            DdcOpcode::ResetVcp => 0x09,
            DdcOpcode::TableReadRequest => 0xe2,
            DdcOpcode::TableReadReply => 0xe4,
            DdcOpcode::TableWrite => 0xe7,
            DdcOpcode::EnableApplicationReport => 0xf5,
            DdcOpcode::SaveCurrentSettings => 0x0c,
        }
    }
}

impl From<u8> for DdcOpcode {
    fn from(value: u8) -> Self {
        match value {
            0xf1 => Self::IdentificationRequest,
            0xe1 => Self::IdentificationReply,
            0xf3 => Self::CapabilitiesRequest,
            0xe3 => Self::CapabilitiesReply,
            0xb1 => Self::DisplaySelfTestRequest,
            0xa1 => Self::DisplaySelfTestReply,
            0x07 => Self::TimingRequest,
            0x06 => Self::TimingReply,
            0x01 => Self::VcpRequest,
            0x02 => Self::VcpReply,
            0x03 => Self::SetVcp,
            0x09 => Self::ResetVcp,
            0xe2 => Self::TableReadRequest,
            0xe4 => Self::TableReadReply,
            0xe7 => Self::TableWrite,
            0xf5 => Self::EnableApplicationReport,
            0x0c => Self::SaveCurrentSettings,
            _ => Self::Unknown(value),
        }
    }
}

impl DdcOpcode {
    /// check if opcode requires offset fields, used for parsing
    fn has_offset(&self) -> bool {
        match self {
            DdcOpcode::IdentificationRequest => false,
            DdcOpcode::IdentificationReply => false,
            DdcOpcode::CapabilitiesRequest => true,
            DdcOpcode::CapabilitiesReply => true,
            DdcOpcode::DisplaySelfTestRequest => false,
            DdcOpcode::DisplaySelfTestReply => false,
            DdcOpcode::TimingRequest => false,
            DdcOpcode::TimingReply => false,
            DdcOpcode::VcpRequest => false,
            DdcOpcode::VcpReply => false,
            DdcOpcode::SetVcp => false,
            DdcOpcode::ResetVcp => false, // actually i have no clue here since the standart give no format for this, i assume not
            DdcOpcode::TableReadRequest => true,
            DdcOpcode::TableReadReply => true,
            DdcOpcode::TableWrite => true,
            DdcOpcode::EnableApplicationReport => false,
            DdcOpcode::SaveCurrentSettings => false,
            DdcOpcode::Unknown(_) => {
                // unknown or unimplemented assume no offset values, if there are some they will be present in the data fragment
                false
            }
        }
    }

    /// check if opcode rquires vcp feature field, used for parsing
    fn has_vcp_feature(&self) -> bool {
        match self {
            DdcOpcode::IdentificationRequest => false,
            DdcOpcode::IdentificationReply => false,
            DdcOpcode::CapabilitiesRequest => false,
            DdcOpcode::CapabilitiesReply => false,
            DdcOpcode::DisplaySelfTestRequest => false,
            DdcOpcode::DisplaySelfTestReply => false,
            DdcOpcode::TimingRequest => false,
            DdcOpcode::TimingReply => false,
            DdcOpcode::VcpRequest => true,
            DdcOpcode::VcpReply => false, // the vcp feature is not located as expected this respones should therefor be received in raw form
            DdcOpcode::SetVcp => true,
            DdcOpcode::ResetVcp => unimplemented!(
                "I don't know if reset has a vcp value field, i can not find it in the standard"
            ),
            DdcOpcode::TableReadRequest => true,
            DdcOpcode::TableReadReply => false,
            DdcOpcode::TableWrite => true,
            DdcOpcode::EnableApplicationReport => false,
            DdcOpcode::SaveCurrentSettings => false,
            DdcOpcode::Unknown(_) => {
                // unknown opcode assume no format
                false
            }
        }
    }

    /// return if the opcode is supposed to be a response from the ddc/ci dislay
    fn is_response(&self) -> bool {
        match self {
            DdcOpcode::IdentificationRequest => false,
            DdcOpcode::IdentificationReply => true,
            DdcOpcode::CapabilitiesRequest => false,
            DdcOpcode::CapabilitiesReply => true,
            DdcOpcode::DisplaySelfTestRequest => false,
            DdcOpcode::DisplaySelfTestReply => true,
            DdcOpcode::TimingRequest => false,
            DdcOpcode::TimingReply => true,
            DdcOpcode::VcpRequest => false,
            DdcOpcode::VcpReply => true,
            DdcOpcode::SetVcp => false,
            DdcOpcode::ResetVcp => true,
            DdcOpcode::TableReadRequest => false,
            DdcOpcode::TableReadReply => true,
            DdcOpcode::TableWrite => false,
            DdcOpcode::EnableApplicationReport => false,
            DdcOpcode::SaveCurrentSettings => false,
            DdcOpcode::Unknown(_) => {
                // this part of the code is only relevant when constructing Messages, assume that unknown in this case
                // is supposed to be sent. In case of receive this definition is irrelavant
                false
            }
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum ResultCode {
    NoError,
    UnsupportedCode,
}

fn parse_result_code(i: &[u8]) -> IResult<&[u8], ResultCode> {
    let (i, rc) = le_u8(i)?;
    match rc {
        0x00 => Ok((i, ResultCode::NoError)),
        0x01 => Ok((i, ResultCode::UnsupportedCode)),
        _ => Err(nom::Err::Failure(nom::error::Error::new(
            i,
            nom::error::ErrorKind::Fail,
        ))),
    }
}

#[derive(Debug)]
pub enum VcpType {
    SetParameter,
    Momentary,
}

fn parse_vcp_type(i: &[u8]) -> IResult<&[u8], VcpType> {
    let (i, ty) = le_u8(i)?;
    match ty {
        0x00 => Ok((i, VcpType::SetParameter)),
        0x01 => Ok((i, VcpType::Momentary)),
        _ => Err(nom::Err::Failure(nom::error::Error::new(
            i,
            nom::error::ErrorKind::Fail,
        ))),
    }
}

#[derive(Debug)]
pub struct FeatureReplyMessage {
    result_code: ResultCode,
    vcp_feature: VcpFeatureCode,
    type_code: VcpType,
    vcp_data: u32,
}

impl FeatureReplyMessage {
    pub fn result_code(&self) -> &ResultCode {
        &self.result_code
    }

    pub fn vcp_feature(&self) -> VcpFeatureCode {
        self.vcp_feature
    }

    pub fn vcp_data(&self) -> u32 {
        self.vcp_data
    }

    pub fn type_code(&self) -> &VcpType {
        &self.type_code
    }
}

pub fn parse_feature_reply(i: &[u8]) -> IResult<&[u8], FeatureReplyMessage> {
    let (i, rc) = parse_result_code(i)?;
    let (i, vcp_code) = le_u8(i)?;
    let (i, tp) = parse_vcp_type(i)?;
    let (i, mh) = le_u8(i)?;
    let (i, ml) = le_u8(i)?;
    let (i, vh) = le_u8(i)?;
    let (i, vl) = le_u8(i)?;
    Ok((
        i,
        FeatureReplyMessage {
            result_code: rc,
            vcp_feature: vcp_code.into(),
            type_code: tp,
            vcp_data: (mh as u32) << 24 | (ml as u32) << 16 | (vh as u32) << 8 | vl as u32,
        },
    ))
}

#[derive(Debug, PartialEq)]
pub struct DdcCiMessage {
    target: u8,
    sender: u8,
    opcode: Option<DdcOpcode>,
    vcp_feature: Option<VcpFeatureCode>,
    offset: Option<u16>,
    data_length: u8,
    data: [u8; DDC_MAX_DATA_FRAGMENT_LENGTH_WITH_EXTRA],
}

impl DdcCiMessage {
    fn protocol_length(&self) -> u8 {
        let mut length = self.data_length;
        if self.opcode.is_some() {
            length += 1;
        }
        if self.vcp_feature.is_some() {
            length += 1;
        }
        if self.offset.is_some() {
            length += 2;
        }
        length
    }

    fn compute_checksum(&self) -> u8 {
        let mut checksum = if self.target == DDC_SLAVE_SEND_ADDR {
            DDC_MASTER_RECV_ADDR
        } else {
            self.target
        };
        checksum ^= self.sender;
        checksum ^= LENGTH_PREFIX | self.protocol_length();
        if let Some(opcode) = &self.opcode {
            checksum ^= Into::<u8>::into(opcode);
        }
        if let Some(vcp_feature) = &self.vcp_feature {
            checksum ^= Into::<u8>::into(*vcp_feature);
        }
        if let Some(offset) = &self.offset {
            checksum ^= offset.to_le_bytes()[1];
            checksum ^= offset.to_le_bytes()[0];
        }
        for i in 0..self.data_length {
            checksum ^= self.data[i as usize];
        }
        checksum
    }

    #[allow(non_snake_case)]
    pub fn NullResponse() -> Self {
        Self {
            target: DDC_SLAVE_SEND_ADDR,
            sender: DDC_SLAVE_RECV_ADDR,
            opcode: None,
            vcp_feature: None,
            offset: None,
            data_length: 0,
            data: [0; DDC_MAX_DATA_FRAGMENT_LENGTH_WITH_EXTRA],
        }
    }

    pub fn from_opcode(opcode: DdcOpcode) -> Self {
        Self {
            target: if opcode.is_response() {
                DDC_SLAVE_SEND_ADDR
            } else {
                DDC_SLAVE_RECV_ADDR
            },
            sender: if opcode.is_response() {
                DDC_SLAVE_RECV_ADDR
            } else {
                DDC_MASTER_SEND_ADDR
            },
            opcode: Some(opcode),
            vcp_feature: None,
            offset: None,
            data_length: 0,
            data: [0; DDC_MAX_DATA_FRAGMENT_LENGTH_WITH_EXTRA],
        }
    }

    pub fn get_opcode(&self) -> Option<&DdcOpcode> {
        self.opcode.as_ref()
    }

    pub fn set_vcp_feature(mut self, feature: VcpFeatureCode) -> Self {
        self.vcp_feature = Some(feature);
        self
    }

    pub fn set_offset(mut self, offset: u16) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn add_offset(mut self, add_offest: u8) -> Self {
        if let Some(offset) = self.offset {
            self.offset = Some(offset + add_offest as u16)
        } else {
            self.offset = Some(add_offest as u16)
        }
        self
    }

    pub fn get_offset(&self) -> Option<u16> {
        self.offset
    }

    pub fn set_data(mut self, data: &[u8]) -> Result<Self, DdcCiProtocolError> {
        if data.len() > DDC_MAX_DATA_FRAGMENT_LENGTH {
            return Err(DdcCiProtocolError::InvalidLength);
        }
        self.data_length = data.len() as u8;
        for i in 0..data.len() {
            self.data[i] = data[i];
        }
        Ok(self)
    }

    pub fn get_data(&self) -> &[u8] {
        &self.data[0..self.data_length as usize]
    }

    pub fn get_data_len(&self) -> u8 {
        self.data_length
    }

    pub fn addr(&self) -> u8 {
        self.target >> 1
    }

    pub fn transmit_buffer(&self) -> Vec<u8> {
        // sender field is not part of protocol length so we need one extra byte here
        let mut data = Vec::with_capacity((self.protocol_length() + 1).into());
        data.push(self.sender);
        data.push(LENGTH_PREFIX | self.protocol_length());
        if let Some(opcode) = &self.opcode {
            data.push(Into::<u8>::into(opcode));
        }
        if let Some(vcp_feature) = &self.vcp_feature {
            data.push(Into::<u8>::into(*vcp_feature));
        }
        if let Some(offset) = &self.offset {
            data.push(offset.to_le_bytes()[1]);
            data.push(offset.to_le_bytes()[0]);
        }
        for j in 0..self.data_length {
            data.push(self.data[j as usize]);
        }
        data.push(self.compute_checksum());
        data
    }

    pub fn parse_buffer(data: &[u8]) -> Result<Self, DdcCiProtocolError> {
        let (i, target) = le_u8::<&[u8], nom::error::Error<_>>.parse(data)?;
        let (i, sender) = le_u8::<&[u8], nom::error::Error<_>>.parse(i)?;
        let (i, maybe_length) = peek(le_u8::<&[u8], nom::error::Error<_>>).parse(i)?;
        if (maybe_length & LENGTH_PREFIX) == LENGTH_PREFIX {
            // this is the most expected case, the field is the length field
            let (i, length) = le_u8::<&[u8], nom::error::Error<_>>.parse(i)?;
            let mut length = length & 0x7f; // extract relevant length bits from byte
            let mut message = Self {
                target,
                sender,
                opcode: None,
                vcp_feature: None,
                offset: None,
                data_length: 0,
                data: [0; DDC_MAX_DATA_FRAGMENT_LENGTH_WITH_EXTRA],
            };
            let i = if length > 0 {
                let (i, opcode) = le_u8::<&[u8], nom::error::Error<_>>.parse(i)?;
                let mut rest_data = i;
                let opcode: DdcOpcode = opcode.into();
                length -= 1;
                // check for opcode relevant vcp feature
                if opcode.has_vcp_feature() && length >= 1 {
                    let (i, vcp_feature) = le_u8::<&[u8], nom::error::Error<_>>.parse(rest_data)?;
                    rest_data = i;
                    length -= 1;
                    message = message.set_vcp_feature(vcp_feature.into());
                }
                // check for opcode relevant offset data
                if opcode.has_offset() && length >= 2 {
                    let (i, offset_high) = le_u8::<&[u8], nom::error::Error<_>>.parse(rest_data)?;
                    let (i, offset_low) = le_u8::<&[u8], nom::error::Error<_>>.parse(i)?;
                    rest_data = i;
                    length -= 2;
                    let offset: u16 = (offset_high as u16) << 8 | offset_low as u16;
                    message = message.set_offset(offset);
                }
                message.opcode = Some(opcode);
                // rest of length should be message data
                message.data_length = length;
                for j in 0..length {
                    let (i, x) = le_u8::<&[u8], nom::error::Error<_>>.parse(rest_data)?;
                    rest_data = i;
                    message.data[j as usize] = x;
                }
                rest_data
            } else {
                i
            };
            let (_i, check_sum) = le_u8::<&[u8], nom::error::Error<_>>.parse(i)?;
            if check_sum == message.compute_checksum() {
                Ok(message)
            } else {
                Err(DdcCiProtocolError::InvalidChecksum)
            }
        } else if maybe_length == (&DdcOpcode::TimingReply).into() {
            todo!()
        } else {
            Err(DdcCiProtocolError::InvalidMessageFormat)
        }
    }
}

impl<T> From<nom::Err<T>> for DdcCiProtocolError
where
    T: core::fmt::Debug,
{
    fn from(value: nom::Err<T>) -> Self {
        DdcCiProtocolError::ParserError(format!("{value:?}"))
    }
}

#[cfg(test)]
mod test {
    use crate::ddc::ci::{DDC_SLAVE_RECV_ADDR, DDC_SLAVE_SEND_ADDR};

    use super::DdcCiMessage;

    struct TestCiMessage {
        data: Vec<u8>,
    }

    impl TestCiMessage {
        pub fn response(addr: u8, data: &[u8]) -> Self {
            let mut msg_data = Vec::with_capacity(data.len() + 1);
            msg_data.push(addr << 1 | 0x01);
            for x in data.iter() {
                msg_data.push(*x);
            }
            Self { data: msg_data }
        }
        #[allow(dead_code)]
        pub fn request(addr: u8, data: &[u8]) -> Self {
            let mut msg_data = Vec::with_capacity(data.len() + 1);
            msg_data.push(addr << 1);
            for x in data.iter() {
                msg_data.push(*x);
            }
            Self { data: msg_data }
        }
    }

    #[test]
    fn null_message_check() {
        let null_msg = DdcCiMessage::NullResponse();
        let test = TestCiMessage::response(null_msg.addr(), &null_msg.transmit_buffer());

        assert_eq!(
            test.data,
            vec![DDC_SLAVE_SEND_ADDR, DDC_SLAVE_RECV_ADDR, 0x80, 0xbe]
        );
    }

    #[test]
    fn parse_null_message() {
        let null_msg = DdcCiMessage::NullResponse();
        let test = TestCiMessage::response(null_msg.addr(), &null_msg.transmit_buffer());

        match DdcCiMessage::parse_buffer(&test.data) {
            Ok(recv_msg) => assert_eq!(recv_msg, null_msg),
            Err(_) => {
                assert!(false)
            }
        }
    }
}
