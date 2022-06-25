use nom::{
    branch::alt,
    bytes::complete::{
        is_not, tag, tag_no_case, take, take_till, take_till1, take_while, take_while1,
    },
    character::{
        complete::{digit1, hex_digit1, line_ending, space0, space1},
        is_space,
    },
    combinator::{map, map_opt, verify},
    error::{ParseError, VerboseError},
    multi::{fold_many0, many0},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    IResult, ParseTo,
};

use crate::{stringesc::StringLosslessExt, stringutil::CharClassExt};

// ------ \/ HTTP start lines \/ ------

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

// ----- \/ quoted-string \/ ------

enum QuotedStringBodyFragment<'a> {
    Literal(&'a [u8]),
    Escaped(&'a [u8]),
}

fn quoted_string_literal<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], &'a [u8], E>
where
    E: ParseError<&'a [u8]>,
{
    take_while1(|c: u8| c.is_text_ws() && !b"\"\\".contains(&c))(input)
}

fn quoted_pair_octet<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], &'a [u8], E>
where
    E: ParseError<&'a [u8]>,
{
    verify(take(1usize), |bytes: &[u8]| bytes[0].is_text_ws())(input)
}

fn quoted_string_escaped<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], &'a [u8], E>
where
    E: ParseError<&'a [u8]>,
{
    preceded(tag(b"\\"), quoted_pair_octet)(input)
}

fn quoted_string_body<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], QuotedStringBodyFragment, E>
where
    E: ParseError<&'a [u8]>,
{
    alt((
        map(quoted_string_escaped, QuotedStringBodyFragment::Escaped),
        map(quoted_string_literal, QuotedStringBodyFragment::Literal),
    ))(input)
}

fn quoted_string<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], Vec<u8>, E>
where
    E: ParseError<&'a [u8]>,
{
    let build_string = fold_many0(quoted_string_body, Vec::new, |mut buf, fragment| {
        match fragment {
            QuotedStringBodyFragment::Literal(v) => buf.extend_from_slice(v),
            QuotedStringBodyFragment::Escaped(v) => buf.extend_from_slice(v),
        }

        buf
    });

    delimited(tag(b"\""), build_string, tag(b"\""))(input)
}

pub fn parse_quoted_string(input: &[u8]) -> Result<String, nom::Err<VerboseError<&[u8]>>> {
    let output = quoted_string::<VerboseError<&[u8]>>(input)?;
    Ok(String::from_utf8_lossless(&output.1))
}

// ----- \/ parameter \/ ------

fn parameter_name<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], &'a [u8], E>
where
    E: ParseError<&'a [u8]>,
{
    take_while1(|c: u8| c.is_token())(input)
}

fn parameter_value<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], Vec<u8>, E>
where
    E: ParseError<&'a [u8]>,
{
    alt((
        quoted_string,
        map(take_while(|c: u8| c.is_token()), |item: &[u8]| {
            item.to_vec()
        }),
    ))(input)
}

fn parameter<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], (&'a [u8], Vec<u8>), E>
where
    E: ParseError<&'a [u8]>,
{
    separated_pair(
        parameter_name,
        tuple((space0, tag("="), space0)),
        parameter_value,
    )(input)
}

pub fn parse_parameter(input: &[u8]) -> Result<(String, String), nom::Err<VerboseError<&[u8]>>> {
    let output = parameter::<VerboseError<&[u8]>>(input)?;
    let pair = output.1;
    Ok((
        crate::stringutil::decode_and_trim_to_string(pair.0),
        String::from_utf8_lossless(&pair.1),
    ))
}

// ----- \/ chunked transfer coding \/ ------

type ChunkLine = (u64, Vec<ChunkExtPair>);
type ChunkExtPair = (String, String);

fn chunk_size<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], u64, E>
where
    E: ParseError<&'a [u8]>,
{
    map_opt(hex_digit1, |item| {
        u64::from_str_radix(&String::from_utf8_lossy(item), 16).ok()
    })(input)
}

fn chunk_ext_name_only<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], ChunkExtPair, E>
where
    E: ParseError<&'a [u8]>,
{
    map(is_not(";"), |item| {
        (
            crate::stringutil::decode_and_trim_to_string(item),
            String::new(),
        )
    })(input)
}

fn chunk_ext_parameter<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], ChunkExtPair, E>
where
    E: ParseError<&'a [u8]>,
{
    map(parameter, |item| {
        (
            crate::stringutil::decode_and_trim_to_string(item.0),
            String::from_utf8_lossless(&item.1),
        )
    })(input)
}

fn chunk_ext<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], ChunkExtPair, E>
where
    E: ParseError<&'a [u8]>,
{
    preceded(
        tuple((space0, tag(b";"), space0)),
        alt((chunk_ext_parameter, chunk_ext_name_only)),
    )(input)
}

fn chunk_exts<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], Vec<ChunkExtPair>, E>
where
    E: ParseError<&'a [u8]>,
{
    many0(chunk_ext)(input)
}

fn chunk_line<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], (u64, Vec<ChunkExtPair>), E>
where
    E: ParseError<&'a [u8]>,
{
    terminated(pair(chunk_size, chunk_exts), line_ending)(input)
}

fn chunk_line_fallback<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], u64, E>
where
    E: ParseError<&'a [u8]>,
{
    chunk_size(input)
}

pub fn parse_chunk_line(input: &[u8]) -> Result<ChunkLine, nom::Err<VerboseError<&[u8]>>> {
    let result = chunk_line::<VerboseError<&[u8]>>(input)?;
    Ok(result.1)
}

pub fn parse_chunk_line_fallback(input: &[u8]) -> Result<u64, nom::Err<VerboseError<&[u8]>>> {
    let result = chunk_line_fallback::<VerboseError<&[u8]>>(input)?;
    Ok(result.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quoted_string() {
        let data = b"\"\"";
        let result = parse_quoted_string(data).unwrap();
        assert_eq!(result, "");

        let data = b"\"Hello world!\"";
        let result = parse_quoted_string(data).unwrap();
        assert_eq!(result, "Hello world!");

        let data = b"\" \\\" \"";
        let result = parse_quoted_string(data).unwrap();
        assert_eq!(result, " \" ");

        let data = b"\" \\\xF0\x9F\x98\x80 \"";
        let result = parse_quoted_string(data).unwrap();
        assert_eq!(result, " 😀 ");
    }

    #[test]
    fn test_parameter() {
        let data = b"k1=v1";
        let result = parse_parameter(data).unwrap();
        assert_eq!(result.0, "k1");
        assert_eq!(result.1, "v1");

        let data = b"k1 = v1";
        let result = parse_parameter(data).unwrap();
        assert_eq!(result.0, "k1");
        assert_eq!(result.1, "v1");

        let data = b"k1=\"hello world!\"";
        let result = parse_parameter(data).unwrap();
        assert_eq!(result.0, "k1");
        assert_eq!(result.1, "hello world!");
    }

    #[test]
    fn test_parse_chunk_line_simple() {
        let (size, exts) = parse_chunk_line(b"05\r\n").unwrap();

        assert_eq!(size, 5);
        assert_eq!(exts.len(), 0);
    }

    #[test]
    fn test_parse_chunk_line_exts() {
        let (size, exts) = parse_chunk_line(b"05 ; p1 ; p2=v2 ; p3 = \"v3\"\r\n").unwrap();

        assert_eq!(size, 5);
        assert_eq!(exts.len(), 3);

        assert_eq!(exts[0].0, "p1");
        assert_eq!(exts[0].1, "");
        assert_eq!(exts[1].0, "p2");
        assert_eq!(exts[1].1, "v2");
        assert_eq!(exts[2].0, "p3");
        assert_eq!(exts[2].1, "v3");
    }

    #[test]
    fn test_parse_chunk_line_fallback() {
        let size = parse_chunk_line_fallback(b"05 \x00\r\n").unwrap();

        assert_eq!(size, 5);
    }
}
