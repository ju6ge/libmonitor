use crate::{
    ddc::ci::DdcOpcode,
    mccs::features::{DiscreteValues, VcpCapability, VcpCapabilityError, VcpFeatureCode},
};

use super::{bracketed, map_err, trim_spaces, OResult, Value, ValueParser};
use nom::{
    branch::alt,
    bytes::complete::take,
    character::complete::{char, u8},
    combinator::{all_consuming, map, map_parser, map_res, opt, rest},
    multi::many0,
    sequence::{separated_pair, tuple},
    Finish, IResult, Parser,
};
use std::{io, str};

/// Parsed display capabilities string entry
#[derive(Clone, Debug, PartialEq)]
pub enum Cap<'a> {
    Protocol(&'a str),
    Type(&'a str),
    Model(&'a str),
    Commands(Vec<DdcOpcode>),
    Whql(u8),
    MccsVersion(u8, u8),
    Vcp(Vec<VcpCapability>),
    Unknown(Value<'a>),
}

impl<'i> Cap<'i> {
    pub fn parse_entries(
        entries: ValueParser<'i>,
    ) -> impl Iterator<Item = io::Result<Cap<'i>>> + 'i {
        entries
            .nom_iter()
            .map(|e| e.and_then(|e| Self::parse_entry(e)).map_err(map_err))
    }

    pub fn parse_entry(value: Value<'i>) -> OResult<'i, Cap<'i>> {
        match value {
            Value::String { tag, value } => Self::parse_string(tag, value),
            Value::Binary { tag, data } => Ok(Self::parse_data(tag, data)),
        }
    }

    pub fn parse_data(tag: &'i str, i: &'i [u8]) -> Cap<'i> {
        match tag {
            _ => Cap::Unknown(Value::Binary { tag, data: i }),
        }
    }

    pub fn parse_string(tag: &'i str, i: &'i [u8]) -> OResult<'i, Cap<'i>> {
        match tag {
            "prot" => all_consuming(map(value, Cap::Protocol))(i),
            "type" => all_consuming(map(value, Cap::Type))(i),
            "model" => all_consuming(map(value, Cap::Model))(i),
            "cmds" => all_consuming(map(
                hexarray.map(|val| {
                    let mut opcodes: Vec<DdcOpcode> = Vec::new();
                    for v in val {
                        opcodes.push(v.into())
                    }
                    opcodes
                }),
                Cap::Commands,
            ))(i),
            "mswhql" => all_consuming(map(map_parser(take(1usize), u8), Cap::Whql))(i),
            "mccs_ver" => all_consuming(map(mccs_ver, |(major, minor)| {
                Cap::MccsVersion(major, minor)
            }))(i),
            "vcp" | "VCP" => all_consuming(map(trim_spaces(many0(vcp)), Cap::Vcp))(i),
            _ => Ok((
                Default::default(),
                Cap::Unknown(Value::String { tag, value: i }),
            )),
        }
        .finish()
        .map(|(_, c)| c)
    }
}

fn value(i: &[u8]) -> IResult<&[u8], &str> {
    map_res(rest, str::from_utf8)(i)
}

fn hexarray(i: &[u8]) -> IResult<&[u8], Vec<u8>> {
    many0(trim_spaces(hexvalue))(i)
}

fn map_str<'i, O, E2, F, G>(mut parser: F, f: G, i: &'i [u8]) -> IResult<&'i [u8], O>
where
    F: nom::Parser<&'i [u8], &'i [u8], nom::error::Error<&'i [u8]>>,
    G: FnMut(&'i str) -> Result<O, E2>,
{
    use nom::Parser;

    let mut f = map_res(rest, f);
    let (i, s) = map_res(|i| parser.parse(i), |i| str::from_utf8(i.into()))(i)?;
    match f.parse(s) {
        Ok((_, v)) => Ok((i, v)),
        Err(e) => Err(e.map(|e: nom::error::Error<_>| nom::error::Error {
            input: i,
            code: e.code,
        })),
    }
}

fn hexvalue(i: &[u8]) -> IResult<&[u8], u8> {
    map_str(take(2usize), |s| u8::from_str_radix(s, 16), i)
}

fn vcp(i: &[u8]) -> IResult<&[u8], VcpCapability> {
    let (i, (code, values)) = tuple((
        trim_spaces(hexvalue),
        opt(bracketed(many0(trim_spaces(hexvalue)))),
    ))(i)?;
    let code: VcpFeatureCode = code.into();
    let mut vcp_cap = match VcpCapability::from_feature_code(code) {
        Ok(x) => x,
        Err(VcpCapabilityError::UnknownCapability) => {
            unreachable!("While pasing an u8 it is not possible to get the unknown variant of the feature code tuple")
        }
        Err(VcpCapabilityError::UnimplementedVcpMapping) => {
            if values.is_some() {
                VcpCapability::UnimplementedDiscrete((code, DiscreteValues::default()))
            } else {
                VcpCapability::Unimplemented(code)
            }
        }
    };

    if let Some(values) = values {
        for v in values {
            vcp_cap.add_discrete_value(v as u32);
        }
    }
    Ok((i, vcp_cap))
}

fn mccs_ver(i: &[u8]) -> IResult<&[u8], (u8, u8)> {
    alt((
        separated_pair(u8, char('.'), u8),
        tuple((map_parser(take(2usize), u8), map_parser(take(2usize), u8))),
    ))(i)
}
