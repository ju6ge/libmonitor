//! VESA Monitor Command Control Set standardizes the meaning of DDC/CI VCP
//! feature codes, and allows a display to broadcast its capabilities to the
//! host.

pub mod capabilities;
pub mod features;

use std::{
    convert::Infallible,
    fmt::{self, Display, Formatter},
    str::FromStr,
};

/// Display protocol class
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Protocol {
    /// Standard monitor
    Monitor,
    /// I have never seen this outside of an MCCS spec example, it may be a typo.
    Display,
    /// Unrecognized protocol class
    Unknown(String),
}

impl<'a> From<&'a str> for Protocol {
    fn from(s: &'a str) -> Self {
        match s {
            "monitor" => Protocol::Monitor,
            "display" => Protocol::Display,
            s => Protocol::Unknown(s.into()),
        }
    }
}

impl Display for Protocol {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(
            match *self {
                Protocol::Monitor => "monitor",
                Protocol::Display => "display",
                Protocol::Unknown(ref s) => s,
            },
            f,
        )
    }
}

impl FromStr for Protocol {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.into())
    }
}

/// Display type
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DisplayTechnology {
    /// Cathode Ray Tube display
    Crt,
    /// Liquid Crystal Display
    Lcd,
    /// Oled
    Led,
    /// Unrecognized display type
    Unknown(String),
}

impl<'a> From<&'a str> for DisplayTechnology {
    fn from(s: &'a str) -> Self {
        match s {
            s if s.eq_ignore_ascii_case("crt") => DisplayTechnology::Crt,
            s if s.eq_ignore_ascii_case("lcd") => DisplayTechnology::Lcd,
            s if s.eq_ignore_ascii_case("led") => DisplayTechnology::Led,
            s => DisplayTechnology::Unknown(s.into()),
        }
    }
}

impl Display for DisplayTechnology {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(
            match *self {
                DisplayTechnology::Crt => "crt",
                DisplayTechnology::Lcd => "lcd",
                DisplayTechnology::Led => "led",
                DisplayTechnology::Unknown(ref s) => s,
            },
            f,
        )
    }
}

impl FromStr for DisplayTechnology {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.into())
    }
}

/// Monitor Command Control Set specification version code
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    /// Major version number
    pub major: u8,
    /// Minor revision version
    pub minor: u8,
}

impl Version {
    /// Create a new MCCS version from the specified version and revision.
    pub fn new(major: u8, minor: u8) -> Self {
        Version { major, minor }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// An unrecognized entry in the capability string
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnknownTag {
    /// The name of the entry
    pub name: String,
    /// The data contained in the entry, usually an unparsed string.
    pub data: UnknownData,
}

/// Data that can be contained in a capability entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnknownData {
    /// UTF-8/ASCII data
    String(String),
    /// Data that is not valid UTF-8
    StringBytes(Vec<u8>),
    /// Length-prefixed binary data
    Binary(Vec<u8>),
}
