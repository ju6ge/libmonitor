use nom::bytes::complete::{tag, take};
use nom::combinator::peek;
use nom::multi::{count, fill};
use nom::number::complete::{be_u16, le_u16, le_u32, le_u8};
use nom::{IResult, Parser};
use thiserror::Error;

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Header {
    pub vendor: [char; 3],
    pub product: u16,
    pub serial: u32,
    pub week: u8,
    pub year: u8, // Starting at year 1990
    pub version: u8,
    pub revision: u8,
}

fn parse_vendor(v: u16) -> [char; 3] {
    let mask: u8 = 0x1F; // Each letter is 5 bits
    let i0 = ('A' as u8) - 1; // 0x01 = A
    return [
        (((v >> 10) as u8 & mask) + i0) as char,
        (((v >> 5) as u8 & mask) + i0) as char,
        (((v >> 0) as u8 & mask) + i0) as char,
    ];
}

fn parse_header(i: &[u8]) -> IResult<&[u8], Header> {
    let (i, _) = tag(&[0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]).parse(i)?;
    let (i, vendor) = be_u16.parse(i)?;
    let (i, product) = le_u16.parse(i)?;
    let (i, serial) = le_u32.parse(i)?;
    let (i, week) = le_u8.parse(i)?;
    let (i, year) = le_u8.parse(i)?;
    let (i, version) = le_u8.parse(i)?;
    let (i, revision) = le_u8.parse(i)?;
    Ok((
        i,
        Header {
            vendor: parse_vendor(vendor),
            product,
            serial,
            week,
            year,
            version,
            revision,
        },
    ))
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Display {
    pub video_input: u8,
    pub width: u8,  // cm
    pub height: u8, // cm
    pub gamma: u8,  // datavalue = (gamma*100)-100 (range 1.00–3.54)
    pub features: u8,
}

fn parse_display(i: &[u8]) -> IResult<&[u8], Display> {
    let (i, video_input) = le_u8.parse(i)?;
    let (i, width) = le_u8.parse(i)?;
    let (i, height) = le_u8.parse(i)?;
    let (i, gamma) = le_u8.parse(i)?;
    let (i, features) = le_u8.parse(i)?;
    Ok((
        i,
        Display {
            video_input,
            width,
            height,
            gamma,
            features,
        },
    ))
}

fn parse_chromaticity(i: &[u8]) -> IResult<&[u8], ()> {
    let (i, _) = take(10 as usize).parse(i)?;
    Ok((i, ()))
}

fn parse_established_timing(i: &[u8]) -> IResult<&[u8], ()> {
    let (i, _) = take(3 as usize).parse(i)?;
    Ok((i, ()))
}

fn parse_standard_timing(i: &[u8]) -> IResult<&[u8], ()> {
    let (i, _) = take(16 as usize).parse(i)?;
    Ok((i, ()))
}

fn parse_descriptor_text(i: &[u8]) -> IResult<&[u8], String> {
    let (i, encoded) = take(13 as usize).parse(i)?;
    let decoded = encoded
        .iter()
        .filter(|b| **b != 0x0A)
        .map(|b| cp437_forward(*b))
        .collect::<String>();
    Ok((i, decoded.trim().to_string()))
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct DetailedTiming {
    /// Pixel clock in kHz.
    pub pixel_clock: u32,
    pub horizontal_active_pixels: u16,
    pub horizontal_blanking_pixels: u16,
    pub vertical_active_lines: u16,
    pub vertical_blanking_lines: u16,
    pub horizontal_front_porch: u16,
    pub horizontal_sync_width: u16,
    pub vertical_front_porch: u16,
    pub vertical_sync_width: u16,
    /// Horizontal size in millimeters
    pub horizontal_size: u16,
    /// Vertical size in millimeters
    pub vertical_size: u16,
    /// Border pixels on one side of screen (i.e. total number is twice this)
    pub horizontal_border_pixels: u8,
    /// Border pixels on one side of screen (i.e. total number is twice this)
    pub vertical_border_pixels: u8,
    pub features: u8, /* TODO add enums etc. */
}

fn parse_detailed_timing(i: &[u8]) -> IResult<&[u8], DetailedTiming> {
    let (i, pixel_clock_10khz) = le_u16.parse(i)?;
    let (i, horizontal_active_lo) = le_u8.parse(i)?;
    let (i, horizontal_blanking_lo) = le_u8.parse(i)?;
    let (i, horizontal_px_hi) = le_u8.parse(i)?;
    let (i, vertical_active_lo) = le_u8.parse(i)?;
    let (i, vertical_blanking_lo) = le_u8.parse(i)?;
    let (i, vertical_px_hi) = le_u8.parse(i)?;
    let (i, horizontal_front_porch_lo) = le_u8.parse(i)?;
    let (i, horizontal_sync_width_lo) = le_u8.parse(i)?;
    let (i, vertical_lo) = le_u8.parse(i)?;
    let (i, porch_sync_hi) = le_u8.parse(i)?;
    let (i, horizontal_size_lo) = le_u8.parse(i)?;
    let (i, vertical_size_lo) = le_u8.parse(i)?;
    let (i, size_hi) = le_u8.parse(i)?;
    let (i, horizontal_border) = le_u8.parse(i)?;
    let (i, vertical_border) = le_u8.parse(i)?;
    let (i, features) = le_u8.parse(i)?;
    Ok((
        i,
        DetailedTiming {
            pixel_clock: pixel_clock_10khz as u32 * 10,
            horizontal_active_pixels: (horizontal_active_lo as u16)
                | (((horizontal_px_hi >> 4) as u16) << 8),
            horizontal_blanking_pixels: (horizontal_blanking_lo as u16)
                | (((horizontal_px_hi & 0xf) as u16) << 8),
            vertical_active_lines: (vertical_active_lo as u16)
                | (((vertical_px_hi >> 4) as u16) << 8),
            vertical_blanking_lines: (vertical_blanking_lo as u16)
                | (((vertical_px_hi & 0xf) as u16) << 8),
            horizontal_front_porch: (horizontal_front_porch_lo as u16)
                | (((porch_sync_hi >> 6) as u16) << 8),
            horizontal_sync_width: (horizontal_sync_width_lo as u16)
                | ((((porch_sync_hi >> 4) & 0x3) as u16) << 8),
            vertical_front_porch: ((vertical_lo >> 4) as u16)
                | ((((porch_sync_hi >> 2) & 0x3) as u16) << 8),
            vertical_sync_width: ((vertical_lo & 0xf) as u16)
                | (((porch_sync_hi & 0x3) as u16) << 8),
            horizontal_size: (horizontal_size_lo as u16) | (((size_hi >> 4) as u16) << 8),
            vertical_size: (vertical_size_lo as u16) | (((size_hi & 0xf) as u16) << 8),
            horizontal_border_pixels: horizontal_border,
            vertical_border_pixels: vertical_border,
            features,
        },
    ))
}

#[derive(Debug, PartialEq, Clone)]
pub enum Descriptor {
    DetailedTiming(DetailedTiming),
    SerialNumber(String),
    UnspecifiedText(String),
    RangeLimits, // TODO
    ProductName(String),
    WhitePoint,     // TODO
    StandardTiming, // TODO
    ColorManagement,
    TimingCodes,
    EstablishedTimings,
    Dummy,
    Unknown([u8; 13]),
}

fn parse_descriptor(i: &[u8]) -> IResult<&[u8], Descriptor> {
    let (i, prefix) = peek(take(3 as usize)).parse(i)?;
    if prefix[0] == 0 && prefix[1] == 0 && prefix[2] == 0 {
        let (i, descriptor_type) = peek(le_u8).parse(i)?;
        match descriptor_type {
            0xFF => {
                let (i, _reserved) = take(5 as usize).parse(i)?;
                let (i, s) = parse_descriptor_text(i)?;
                Ok((i, Descriptor::SerialNumber(s)))
            }
            0xFE => {
                let (i, _reserved) = take(5 as usize).parse(i)?;
                let (i, s) = parse_descriptor_text(i)?;
                Ok((i, Descriptor::UnspecifiedText(s)))
            }
            0xFD => {
                let (i, _reserved) = take(5 as usize).parse(i)?;
                let (i, _data) = take(13 as usize).parse(i)?; //TODO
                Ok((i, Descriptor::RangeLimits))
            }
            0xFC => {
                let (i, _reserved) = take(5 as usize).parse(i)?;
                let (i, s) = parse_descriptor_text(i)?;
                Ok((i, Descriptor::ProductName(s)))
            }
            0xFB => {
                let (i, _reserved) = take(5 as usize).parse(i)?;
                let (i, _data) = take(13 as usize).parse(i)?; //TODO
                Ok((i, Descriptor::WhitePoint))
            }
            0xFA => {
                let (i, _reserved) = take(5 as usize).parse(i)?;
                let (i, _data) = take(13 as usize).parse(i)?; //TODO
                Ok((i, Descriptor::StandardTiming))
            }
            0xF9 => {
                let (i, _reserved) = take(5 as usize).parse(i)?;
                let (i, _data) = take(13 as usize).parse(i)?; //TODO
                Ok((i, Descriptor::ColorManagement))
            }
            0xF8 => {
                let (i, _reserved) = take(5 as usize).parse(i)?;
                let (i, _data) = take(13 as usize).parse(i)?; //TODO
                Ok((i, Descriptor::TimingCodes))
            }
            0xF7 => {
                let (i, _reserved) = take(5 as usize).parse(i)?;
                let (i, _data) = take(13 as usize).parse(i)?; //TODO
                Ok((i, Descriptor::EstablishedTimings))
            }
            0x10 => {
                let (i, _reserved) = take(5 as usize).parse(i)?;
                let (i, _data) = take(13 as usize).parse(i)?; //TODO
                Ok((i, Descriptor::Dummy))
            }
            _ => {
                let (i, _reserved) = take(5 as usize).parse(i)?;
                let mut data = [0; 13];
                let (i, _) = fill(le_u8, &mut data).parse(i)?;
                Ok((i, Descriptor::Unknown(data)))
            }
        }
    } else {
        let (i, timing) = parse_detailed_timing(i)?;
        Ok((i, Descriptor::DetailedTiming(timing)))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Edid {
    pub header: Header,
    pub display: Display,
    chromaticity: (),       // TODO
    established_timing: (), // TODO
    standard_timing: (),    // TODO
    pub descriptors: Vec<Descriptor>,
    pub num_extr: u8,
}

pub fn parse_edid(full_input: &[u8]) -> Result<Edid, EdidParseError> {
    let (i, header) = parse_header(full_input)?;
    let (i, display) = parse_display(i)?;
    let (i, chromaticity) = parse_chromaticity(i)?;
    let (i, established_timing) = parse_established_timing(i)?;
    let (i, standard_timing) = parse_standard_timing(i)?;
    let (i, descriptors) = count(parse_descriptor, 4)(i)?;
    let (i, num_extr) = le_u8::<&[u8], nom::error::Error<_>>.parse(i)?; // number of extensions
    let (_i, check) = le_u8::<&[u8], nom::error::Error<_>>.parse(i)?;
    let mut sum_all: u8 = 0;
    for i in 0..full_input.len() - 1 {
        let (res, _) = sum_all.overflowing_add(full_input[i]);
        sum_all = res;
    }
    if sum_all.overflowing_add(check).0 == 0 {
        Ok(Edid {
            header,
            display,
            chromaticity,
            established_timing,
            standard_timing,
            descriptors,
            num_extr,
        })
    } else {
        Err(EdidParseError::InvalidChecksum)
    }
}

#[derive(Error, Debug)]
pub enum EdidParseError {
    #[error("Checksum is invalid, data corrupt!")]
    InvalidChecksum,
    #[error("Parsing data failed: {0}")]
    NomParserError(String),
}

impl<T> From<nom::Err<T>> for EdidParseError
where
    T: core::fmt::Debug,
{
    fn from(value: nom::Err<T>) -> Self {
        EdidParseError::NomParserError(format!("{value}"))
    }
}

const CP437_FORWARD_TABLE: &'static [u16] = &[
    0x0000, 0x263A, 0x263B, 0x2665, 0x2666, 0x2663, 0x2660, 0x2022, 0x25D8, 0x25CB, 0x25D9, 0x2642,
    0x2640, 0x266A, 0x266B, 0x263C, 0x25BA, 0x25C4, 0x2195, 0x203C, 0x00B6, 0x00A7, 0x25AC, 0x21A8,
    0x2191, 0x2193, 0x2192, 0x2190, 0x221F, 0x2194, 0x25B2, 0x25BC, 0x0020, 0x0021, 0x0022, 0x0023,
    0x0024, 0x0025, 0x0026, 0x0027, 0x0028, 0x0029, 0x002A, 0x002B, 0x002C, 0x002D, 0x002E, 0x002F,
    0x0030, 0x0031, 0x0032, 0x0033, 0x0034, 0x0035, 0x0036, 0x0037, 0x0038, 0x0039, 0x003A, 0x003B,
    0x003C, 0x003D, 0x003E, 0x003F, 0x0040, 0x0041, 0x0042, 0x0043, 0x0044, 0x0045, 0x0046, 0x0047,
    0x0048, 0x0049, 0x004A, 0x004B, 0x004C, 0x004D, 0x004E, 0x004F, 0x0050, 0x0051, 0x0052, 0x0053,
    0x0054, 0x0055, 0x0056, 0x0057, 0x0058, 0x0059, 0x005A, 0x005B, 0x005C, 0x005D, 0x005E, 0x005F,
    0x0060, 0x0061, 0x0062, 0x0063, 0x0064, 0x0065, 0x0066, 0x0067, 0x0068, 0x0069, 0x006A, 0x006B,
    0x006C, 0x006D, 0x006E, 0x006F, 0x0070, 0x0071, 0x0072, 0x0073, 0x0074, 0x0075, 0x0076, 0x0077,
    0x0078, 0x0079, 0x007A, 0x007B, 0x007C, 0x007D, 0x007E, 0x2302, 0x00C7, 0x00FC, 0x00E9, 0x00E2,
    0x00E4, 0x00E0, 0x00E5, 0x00E7, 0x00EA, 0x00EB, 0x00E8, 0x00EF, 0x00EE, 0x00EC, 0x00C4, 0x00C5,
    0x00C9, 0x00E6, 0x00C6, 0x00F4, 0x00F6, 0x00F2, 0x00FB, 0x00F9, 0x00FF, 0x00D6, 0x00DC, 0x00A2,
    0x00A3, 0x00A5, 0x20A7, 0x0192, 0x00E1, 0x00ED, 0x00F3, 0x00FA, 0x00F1, 0x00D1, 0x00AA, 0x00BA,
    0x00BF, 0x2310, 0x00AC, 0x00BD, 0x00BC, 0x00A1, 0x00AB, 0x00BB, 0x2591, 0x2592, 0x2593, 0x2502,
    0x2524, 0x2561, 0x2562, 0x2556, 0x2555, 0x2563, 0x2551, 0x2557, 0x255D, 0x255C, 0x255B, 0x2510,
    0x2514, 0x2534, 0x252C, 0x251C, 0x2500, 0x253C, 0x255E, 0x255F, 0x255A, 0x2554, 0x2569, 0x2566,
    0x2560, 0x2550, 0x256C, 0x2567, 0x2568, 0x2564, 0x2565, 0x2559, 0x2558, 0x2552, 0x2553, 0x256B,
    0x256A, 0x2518, 0x250C, 0x2588, 0x2584, 0x258C, 0x2590, 0x2580, 0x03B1, 0x00DF, 0x0393, 0x03C0,
    0x03A3, 0x03C3, 0x00B5, 0x03C4, 0x03A6, 0x0398, 0x03A9, 0x03B4, 0x221E, 0x03C6, 0x03B5, 0x2229,
    0x2261, 0x00B1, 0x2265, 0x2264, 0x2320, 0x2321, 0x00F7, 0x2248, 0x00B0, 0x2219, 0x00B7, 0x221A,
    0x207F, 0x00B2, 0x25A0, 0x00A0,
];

pub fn cp437_forward(code: u8) -> char {
    char::from_u32(CP437_FORWARD_TABLE[code as usize] as u32).unwrap()
}
