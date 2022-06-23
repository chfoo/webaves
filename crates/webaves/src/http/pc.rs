use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_till, take_till1, take_while1},
    character::{
        complete::{digit1, line_ending, space1},
        is_space,
    },
    combinator::{map, map_opt},
    error::{ParseError, VerboseError},
    sequence::{pair, terminated, tuple},
    IResult, ParseTo,
};

use super::util::HeaderByteExt;

pub struct RequestLine<'a> {
    pub method: &'a [u8],
    pub request_target: &'a [u8],
    pub http_version: (u16, u16),
}

pub struct StatusLine<'a> {
    pub http_version: (u16, u16),
    pub status_code: u16,
    pub reason_phrase: &'a [u8],
}

fn token<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], &'a [u8], E>
where
    E: ParseError<&'a [u8]>,
{
    take_while1(|byte: u8| byte.is_token())(input)
}

fn http_version_int<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], (u16, u16), E>
where
    E: ParseError<&'a [u8]>,
{
    alt((
        map_opt(tuple((digit1, tag("."), digit1)), |(i1, _, i2)| {
            parse_int_2(i1, i2)
        }),
        map_opt(digit1, parse_int_1),
    ))(input)
}

fn parse_int_1(input: &[u8]) -> Option<(u16, u16)> {
    input.parse_to().map(|num| (num, 0))
}

#[allow(clippy::unnecessary_unwrap)]
fn parse_int_2(input_1: &[u8], input_2: &[u8]) -> Option<(u16, u16)> {
    let num_1 = input_1.parse_to();
    let num_2 = input_2.parse_to();

    if num_1.is_some() && num_2.is_some() {
        Some((num_1.unwrap(), num_2.unwrap()))
    } else {
        None
    }
}

fn http_version<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], (u16, u16), E>
where
    E: ParseError<&'a [u8]>,
{
    map(pair(tag_no_case("HTTP/"), http_version_int), |pair| pair.1)(input)
}

fn method<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], &'a [u8], E>
where
    E: ParseError<&'a [u8]>,
{
    token(input)
}

fn request_target<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], &'a [u8], E>
where
    E: ParseError<&'a [u8]>,
{
    take_till1(is_space)(input)
}

fn status_code<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], u16, E>
where
    E: ParseError<&'a [u8]>,
{
    map_opt(digit1, |item: &[u8]| item.parse_to())(input)
}

fn reason_phrase<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], &'a [u8], E>
where
    E: ParseError<&'a [u8]>,
{
    take_till(|c: u8| c.is_ascii_control())(input)
}

fn request_line<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], RequestLine, E>
where
    E: ParseError<&'a [u8]>,
{
    map(
        terminated(
            tuple((method, space1, request_target, space1, http_version)),
            line_ending,
        ),
        |(method, _, request_target, _, http_version)| RequestLine {
            method,
            request_target,
            http_version,
        },
    )(input)
}

fn status_line<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], StatusLine, E>
where
    E: ParseError<&'a [u8]>,
{
    map(
        terminated(
            tuple((
                http_version,
                tag(b" "),
                status_code,
                tag(b" "),
                reason_phrase,
            )),
            line_ending,
        ),
        |(http_version, _, status_code, _, reason_phrase)| StatusLine {
            http_version,
            status_code,
            reason_phrase,
        },
    )(input)
}

pub fn parse_status_line(input: &[u8]) -> Result<StatusLine, nom::Err<VerboseError<&[u8]>>> {
    let result = status_line::<VerboseError<&[u8]>>(input)?;
    Ok(result.1)
}

pub fn parse_request_line(input: &[u8]) -> Result<RequestLine, nom::Err<VerboseError<&[u8]>>> {
    let result = request_line::<VerboseError<&[u8]>>(input)?;
    Ok(result.1)
}
