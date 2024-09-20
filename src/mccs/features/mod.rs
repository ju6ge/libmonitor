pub mod queue;

use std::fmt::Debug;

use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::ddc::{DdcCiDevice, DdcError};

/// VCP feature code
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VcpFeatureCode {
    /// doubles as return value of ActiveControl when FIFO is empty
    CodePage,
    NewControlValue,
    Luminance,
    Contrast,
    ActiveControl,
    OsdLanguage,
    InputSelect,
    //VendorSpecific(u8),
    Unimplemented(u8),
    Unknown,
}

impl VcpValue for VcpFeatureCode {
    fn vcp_feature() -> VcpFeatureCode {
        VcpFeatureCode::ActiveControl
    }
}

impl From<VcpFeatureCode> for u32 {
    fn from(value: VcpFeatureCode) -> Self {
        let vl: u8 = value.into();
        vl as u32
    }
}

impl From<u32> for VcpFeatureCode {
    fn from(value: u32) -> Self {
        ((value & 0xff) as u8).into()
    }
}

impl From<VcpFeatureCode> for u8 {
    fn from(value: VcpFeatureCode) -> Self {
        match value {
            VcpFeatureCode::CodePage => 0x00,
            VcpFeatureCode::NewControlValue => 0x02,
            VcpFeatureCode::Luminance => 0x10,
            VcpFeatureCode::Contrast => 0x12,
            VcpFeatureCode::ActiveControl => 0x52,
            VcpFeatureCode::InputSelect => 0x60,
            VcpFeatureCode::OsdLanguage => 0xcc,
            //VcpFeatureCode::VendorSpecific(val) => val,
            VcpFeatureCode::Unimplemented(val) => val,
            VcpFeatureCode::Unknown => 0x00,
        }
    }
}

impl From<u8> for VcpFeatureCode {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::CodePage,
            0x02 => Self::NewControlValue,
            0x10 => Self::Luminance,
            0x12 => Self::Contrast,
            0x52 => Self::ActiveControl,
            0x60 => Self::InputSelect,
            0xcc => Self::OsdLanguage,
            _ => Self::Unimplemented(value),
        }
    }
}

#[derive(Debug)]
pub enum VcpFeatureValue {
    CodePage(u32),
    NewControlValue(NewControlValue),
    Luminance(LuminanceValue),
    Contrast(ContrastValue),
    Fifo(VcpFeatureCode),
    OsdLanguage(OsdLanguages),
    InputSelect(InputSource),
    Unimplemented(u8, u32),
}

impl VcpFeatureValue {
    pub fn read_from_ddc<D: DdcCiDevice>(ddc_channel: &mut D, feature: VcpFeatureCode) -> Result<Self, DdcError> {
        match feature {
            VcpFeatureCode::CodePage => {
                todo!()
            },
            VcpFeatureCode::NewControlValue => {
                let c: NewControlValue = ddc_channel.get_vcp_feature()?;
                Ok(Self::NewControlValue(c))
            },
            VcpFeatureCode::Luminance => {
                let l: LuminanceValue = ddc_channel.get_vcp_feature()?;
                Ok(Self::Luminance(l))
            },
            VcpFeatureCode::Contrast => {
                let c: ContrastValue = ddc_channel.get_vcp_feature()?;
                Ok(Self::Contrast(c))
            },
            VcpFeatureCode::ActiveControl => {
                todo!()
            },
            VcpFeatureCode::OsdLanguage => {
                let l: OsdLanguages = ddc_channel.get_vcp_feature()?;
                Ok(Self::OsdLanguage(l))
            },
            VcpFeatureCode::InputSelect => {
                let v: InputSource = ddc_channel.get_vcp_feature()?;
                Ok(Self::InputSelect(v))
            },
            VcpFeatureCode::Unimplemented(_) => unimplemented!("Can not read unimplemented feature Code"),
            VcpFeatureCode::Unknown => panic!("Can not read unknow vcp feature!"),
        }
    }
}

// ultimately Vcp Values can contain up to 4 bytes of information
// so we require u32 here for now. Dunno if I will change this again
// depending on further development
pub trait VcpValue: From<u32> + Into<u32> + Copy {
    fn mh(&self) -> u8 {
        let num: u32 = (*self).into();
        (num >> 24 & 0xff) as u8
    }
    fn ml(&self) -> u8 {
        let num: u32 = (*self).into();
        (num >> 16 & 0xff) as u8
    }
    fn vh(&self) -> u8 {
        let num: u32 = (*self).into();
        (num >> 8 & 0xff) as u8
    }
    fn vl(&self) -> u8 {
        let num: u32 = (*self).into();
        (num & 0xff) as u8
    }
    fn vcp_feature() -> VcpFeatureCode;
}

#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct AnonymousVcpValue(u32);
impl VcpValue for AnonymousVcpValue {
    fn vcp_feature() -> VcpFeatureCode {
        VcpFeatureCode::Unknown
    }
}

impl From<u32> for AnonymousVcpValue {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<AnonymousVcpValue> for u32 {
    fn from(value: AnonymousVcpValue) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum NewControlValue {
    NewControlValuesPresent,
    Finished
}

impl From<u32> for NewControlValue {
    fn from(value: u32) -> Self {
        match value & 0x0f {
            0x01 => Self::Finished,
            0x02 => Self::NewControlValuesPresent,
            _ => { panic!("Read VCP New Control Value may on be of values 0x01 or 0x02 not {}", value & 0x0f ) }
        }
    }
}

impl From<NewControlValue> for u32 {
    fn from(value: NewControlValue) -> Self {
        match value {
            NewControlValue::NewControlValuesPresent => 0x02,
            NewControlValue::Finished => 0x01,
        }
    }
}

impl VcpValue for NewControlValue {
    fn vcp_feature() -> VcpFeatureCode {
        VcpFeatureCode::NewControlValue
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LuminanceValue {
    pub max: u16,
    pub val: u16,
}
impl From<u32> for LuminanceValue {
    fn from(value: u32) -> Self {
        Self {
            max: (value >> 16) as u16,
            val: (value & 0xff) as u16,
        }
    }
}
impl From<LuminanceValue> for u32 {
    fn from(value: LuminanceValue) -> Self {
        (value.max as u32) << 16 | value.val as u32
    }
}
impl VcpValue for LuminanceValue {
    fn vcp_feature() -> VcpFeatureCode {
        VcpFeatureCode::Luminance
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ContrastValue {
    pub max: u16,
    pub val: u16,
}
impl ContrastValue {
    pub fn max(&self) -> u16 {
        self.max
    }
    pub fn val(&self) -> u16 {
        self.val
    }
}
impl From<u32> for ContrastValue {
    fn from(value: u32) -> Self {
        Self {
            max: (value >> 16) as u16,
            val: (value & 0xff) as u16,
        }
    }
}
impl From<ContrastValue> for u32 {
    fn from(value: ContrastValue) -> Self {
        (value.max as u32) << 16 | value.val as u32
    }
}
impl VcpValue for ContrastValue {
    fn vcp_feature() -> VcpFeatureCode {
        VcpFeatureCode::Contrast
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscreteValues<V>
where
    V: VcpValue,
{
    discrete_values: Vec<V>,
}

impl<V> Default for DiscreteValues<V>
where
    V: VcpValue,
{
    fn default() -> Self {
        Self {
            discrete_values: Default::default(),
        }
    }
}

impl<V> DiscreteValues<V>
where
    V: VcpValue,
{
    pub fn add_discrete_value(&mut self, val: V) {
        self.discrete_values.push(val);
    }
}

#[derive(PartialEq, Clone)]
pub enum VcpCapability {
    Language(DiscreteValues<OsdLanguages>),
    DisplayInput(DiscreteValues<InputSource>),
    Continuous(VcpFeatureCode),
    UnimplementedDiscrete((VcpFeatureCode, DiscreteValues<AnonymousVcpValue>)),
    Unimplemented(VcpFeatureCode),
}

impl Debug for VcpCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Language(supported_values) => f
                .debug_struct(&format!(
                    "{:?} ({:x})",
                    VcpFeatureCode::OsdLanguage,
                    Into::<u8>::into(VcpFeatureCode::OsdLanguage)
                ))
                .field("possible_values", &supported_values.discrete_values)
                .finish(),
            Self::DisplayInput(supported_values) => f
                .debug_struct(&format!(
                    "{:?} ({:x})",
                    VcpFeatureCode::InputSelect,
                    Into::<u8>::into(VcpFeatureCode::InputSelect)
                ))
                .field("possible_values", &supported_values.discrete_values)
                .finish(),
            Self::UnimplementedDiscrete((code, supported_values)) => f
                .debug_struct(&format!(
                    "VcpFeatureCode ({:x}) unimplemented!",
                    Into::<u8>::into(*code)
                ))
                .field("possible_values", &supported_values.discrete_values)
                .finish(),
            Self::Continuous(feature_code) => f
                .debug_struct(&format!(
                    "{:?} ({:x})",
                    feature_code,
                    Into::<u8>::into(*feature_code)
                ))
                .finish(),
            Self::Unimplemented(feature_code) => f
                .debug_struct(&format!(
                    "VcpFeatureCode ({:x}) unimplemented!",
                    Into::<u8>::into(*feature_code)
                ))
                .finish(),
        }
    }
}

#[derive(Debug, Error)]
pub enum VcpCapabilityError {
    #[error("Can not construct VcpCapability from Feature Code variant unknown!")]
    UnknownCapability,
    #[error(
        "Unimplemented Vcp Mapping please construct type by urself using unimplemented variants"
    )]
    UnimplementedVcpMapping,
}

impl VcpCapability {
    pub fn from_feature_code(code: VcpFeatureCode) -> Result<Self, VcpCapabilityError> {
        match code {
            VcpFeatureCode::OsdLanguage => Ok(Self::Language(Default::default())),
            VcpFeatureCode::InputSelect => Ok(Self::DisplayInput(Default::default())),
            VcpFeatureCode::Unknown => Err(VcpCapabilityError::UnknownCapability),
            VcpFeatureCode::Contrast | VcpFeatureCode::Luminance => Ok(Self::Continuous(code)),
            _ => Err(VcpCapabilityError::UnimplementedVcpMapping),
        }
    }

    pub fn add_discrete_value(&mut self, value: u32) {
        match self {
            VcpCapability::Language(ref mut languages) => {
                languages.add_discrete_value(value.into())
            }
            VcpCapability::DisplayInput(ref mut inputs) => inputs.add_discrete_value(value.into()),
            VcpCapability::UnimplementedDiscrete((_, discrete_values)) => {
                discrete_values.add_discrete_value(value.into())
            }
            _ => { /* notihng to do here, this dose not represent discrete values*/ }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum OsdLanguages {
    Ignored,
    ChineseTraditional,
    English,
    French,
    German,
    Italian,
    Japanese,
    Korean,
    PortuguesePortugal,
    Russian,
    Spanish,
    Swedish,
    Turkish,
    ChineseSimplified,
    PortugueseBrazil,
    Arabic,
    Bulgarian,
    Croatian,
    Czech,
    Danish,
    Dutch,
    Estonian,
    Finnish,
    Greek,
    Hebrew,
    Hindi,
    Hungarian,
    Lativan,
    Lithuanian,
    Norwegian,
    Polish,
    Romanian,
    Serbian,
    Slovak,
    Slovenian,
    Thai,
    Ukrainian,
    Vietnamese,
    UndefinedLanguage(u32),
}

impl From<u32> for OsdLanguages {
    fn from(value: u32) -> Self {
        let mask = 0xffff;
        match value & mask {
            0x0000 => Self::Ignored,
            0x0001 => Self::ChineseTraditional,
            0x0002 => Self::English,
            0x0003 => Self::French,
            0x0004 => Self::German,
            0x0005 => Self::Italian,
            0x0006 => Self::Japanese,
            0x0007 => Self::Korean,
            0x0008 => Self::PortuguesePortugal,
            0x0009 => Self::Russian,
            0x000A => Self::Spanish,
            0x000B => Self::Swedish,
            0x000C => Self::Turkish,
            0x000D => Self::ChineseSimplified,
            0x000E => Self::PortugueseBrazil,
            0x000F => Self::Arabic,
            0x0010 => Self::Bulgarian,
            0x0011 => Self::Croatian,
            0x0012 => Self::Czech,
            0x0013 => Self::Danish,
            0x0014 => Self::Dutch,
            0x0015 => Self::Estonian,
            0x0016 => Self::Finnish,
            0x0017 => Self::Greek,
            0x0018 => Self::Hebrew,
            0x0019 => Self::Hindi,
            0x001A => Self::Hungarian,
            0x001B => Self::Lativan,
            0x001C => Self::Lithuanian,
            0x001D => Self::Norwegian,
            0x001E => Self::Polish,
            0x001F => Self::Romanian,
            0x0020 => Self::Serbian,
            0x0021 => Self::Slovak,
            0x0022 => Self::Slovenian,
            0x0023 => Self::Thai,
            0x0024 => Self::Ukrainian,
            0x0025 => Self::Vietnamese,
            _ => Self::UndefinedLanguage(value & mask),
        }
    }
}

impl From<OsdLanguages> for u32 {
    fn from(value: OsdLanguages) -> Self {
        match value {
            OsdLanguages::Ignored => 0x0000,
            OsdLanguages::ChineseTraditional => 0x0001,
            OsdLanguages::English => 0x0002,
            OsdLanguages::French => 0x0003,
            OsdLanguages::German => 0x0004,
            OsdLanguages::Italian => 0x0005,
            OsdLanguages::Japanese => 0x0006,
            OsdLanguages::Korean => 0x0007,
            OsdLanguages::PortuguesePortugal => 0x0008,
            OsdLanguages::Russian => 0x0009,
            OsdLanguages::Spanish => 0x000A,
            OsdLanguages::Swedish => 0x000B,
            OsdLanguages::Turkish => 0x000C,
            OsdLanguages::ChineseSimplified => 0x000D,
            OsdLanguages::PortugueseBrazil => 0x000E,
            OsdLanguages::Arabic => 0x000F,
            OsdLanguages::Bulgarian => 0x0010,
            OsdLanguages::Croatian => 0x0011,
            OsdLanguages::Czech => 0x0012,
            OsdLanguages::Danish => 0x0013,
            OsdLanguages::Dutch => 0x0014,
            OsdLanguages::Estonian => 0x0015,
            OsdLanguages::Finnish => 0x0016,
            OsdLanguages::Greek => 0x0017,
            OsdLanguages::Hebrew => 0x0018,
            OsdLanguages::Hindi => 0x0019,
            OsdLanguages::Hungarian => 0x001A,
            OsdLanguages::Lativan => 0x001B,
            OsdLanguages::Lithuanian => 0x001C,
            OsdLanguages::Norwegian => 0x001D,
            OsdLanguages::Polish => 0x001E,
            OsdLanguages::Romanian => 0x001F,
            OsdLanguages::Serbian => 0x0020,
            OsdLanguages::Slovak => 0x0021,
            OsdLanguages::Slovenian => 0x0022,
            OsdLanguages::Thai => 0x0023,
            OsdLanguages::Ukrainian => 0x0024,
            OsdLanguages::Vietnamese => 0x0025,
            OsdLanguages::UndefinedLanguage(value) => value,
        }
    }
}

impl VcpValue for OsdLanguages {
    fn vcp_feature() -> VcpFeatureCode {
        VcpFeatureCode::OsdLanguage
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum InputSource {
    Analog1,
    Analog2,
    Dvi1,
    Dvi2,
    Composite1,
    Composite2,
    SVideo1,
    SVideo2,
    Tuner1,
    Tuner2,
    Tuner3,
    Component1,
    Component2,
    Component3,
    DisplayPort1,
    DisplayPort2,
    Hdmi1,
    Hdmi2,
    Reserved(u32),
}

impl From<u32> for InputSource {
    fn from(value: u32) -> Self {
        let mask = 0xff;
        match value & mask {
            0x01 => Self::Analog1,
            0x02 => Self::Analog2,
            0x03 => Self::Dvi1,
            0x04 => Self::Dvi2,
            0x05 => Self::Composite1,
            0x06 => Self::Composite2,
            0x07 => Self::SVideo1,
            0x08 => Self::SVideo2,
            0x09 => Self::Tuner1,
            0x0A => Self::Tuner2,
            0x0B => Self::Tuner3,
            0x0C => Self::Component1,
            0x0D => Self::Component2,
            0x0E => Self::Component3,
            0x0f => Self::DisplayPort1,
            0x10 => Self::DisplayPort2,
            0x11 => Self::Hdmi1,
            0x12 => Self::Hdmi2,
            _ => Self::Reserved(value & mask),
        }
    }
}

impl From<InputSource> for u32 {
    fn from(value: InputSource) -> Self {
        match value {
            InputSource::Analog1 => 0x01,
            InputSource::Analog2 => 0x02,
            InputSource::Dvi1 => 0x03,
            InputSource::Dvi2 => 0x04,
            InputSource::Composite1 => 0x05,
            InputSource::Composite2 => 0x06,
            InputSource::SVideo1 => 0x07,
            InputSource::SVideo2 => 0x08,
            InputSource::Tuner1 => 0x09,
            InputSource::Tuner2 => 0x0A,
            InputSource::Tuner3 => 0x0B,
            InputSource::Component1 => 0x0C,
            InputSource::Component2 => 0x0D,
            InputSource::Component3 => 0x0E,
            InputSource::DisplayPort1 => 0x0f,
            InputSource::DisplayPort2 => 0x10,
            InputSource::Hdmi1 => 0x11,
            InputSource::Hdmi2 => 0x12,
            InputSource::Reserved(value) => value,
        }
    }
}

impl VcpValue for InputSource {
    fn vcp_feature() -> VcpFeatureCode {
        VcpFeatureCode::InputSelect
    }
}
