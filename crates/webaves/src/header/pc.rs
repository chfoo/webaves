use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take, take_until, take_while, take_while1},
    character::complete::{line_ending, space1},
    combinator::{all_consuming, map, verify},
    error::{ParseError, VerboseError},
    multi::{fold_many0, many0},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    FindSubstring, IResult,
};

use crate::{stringesc::StringLosslessExt, stringutil::CharClassExt};

use super::{FieldName, FieldPair, FieldValue, HeaderMap};

struct ModifiedInput<'a> {
    original: &'a [u8],
    modified: Vec<u8>,
}

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

fn parameter_name<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], &'a [u8], E>
where
    E: ParseError<&'a [u8]>,
{
    take_until("=")(input)
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
    separated_pair(parameter_name, tag("="), parameter_value)(input)
}

fn quoted_string_body_unchanged<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], &'a [u8], E>
where
    E: ParseError<&'a [u8]>,
{
    alt((is_not("\\\""), tag(b"\\\"")))(input)
}

#[allow(clippy::type_complexity)]
fn quoted_string_unchanged<'a, E>(
    input: &'a [u8],
) -> IResult<&'a [u8], (&'a [u8], &'a [u8], &'a [u8]), E>
where
    E: ParseError<&'a [u8]>,
{
    tuple((tag(b"\""), quoted_string_body_unchanged, tag(b"\"")))(input)
}

fn encoded_word<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], Vec<u8>, E>
where
    E: ParseError<&'a [u8]>,
{
    delimited(tag(b"=?"), take_until("?="), tag(b"?="))(input)?;

    let index = input.find_substring(b"?=".as_slice()).unwrap();
    let body_len = index + 2;

    match rustyknife::rfc2047::encoded_word(&input[0..body_len]) {
        Ok((_, decoded)) => map(take(body_len), |_| decoded.as_bytes().to_vec())(input),
        Err(_) => map(take(body_len), |output: &[u8]| output.to_vec())(input),
    }
}

fn encoded_word_space<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], &'a [u8], E>
where
    E: ParseError<&'a [u8]>,
{
    match pair(space1, tag(b"=?"))(input) {
        Ok(_) => space1(input),
        Err(e) => Err(e),
    }
}

fn field_name<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], &'a [u8], E>
where
    E: ParseError<&'a [u8]>,
{
    take_until(b":".as_slice())(input)
}

enum FieldValueFragment<'a> {
    Literal(&'a [u8]),
    FoldedSep((&'a [u8], &'a [u8])),
    Quoted((&'a [u8], &'a [u8], &'a [u8])),
    EncodedWord(Vec<u8>),
    EncodedWordSpace(&'a [u8]),
}

fn field_value_literal<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], &'a [u8], E>
where
    E: ParseError<&'a [u8]>,
{
    // TODO: there should be a way to make this more concise.
    let a = is_not(b"\"\r\n".as_slice())(input);
    let b = take_until::<_, _, VerboseError<&[u8]>>(b"=?".as_slice())(input);

    match (a, b) {
        (Ok(a), Ok(b)) => {
            if a.0.len() > b.0.len() {
                // more input remaining for a
                Ok(a)
            } else {
                Ok(b)
            }
        }
        (Ok(a), Err(_)) => Ok(a),
        (Err(_), Ok(b)) => Ok(b),
        (Err(a), Err(_)) => Err(a),
    }
}

fn field_value_folded_sep<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], (&'a [u8], &'a [u8]), E>
where
    E: ParseError<&'a [u8]>,
{
    pair(line_ending, space1)(input)
}

fn field_value_body<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], FieldValueFragment, E>
where
    E: ParseError<&'a [u8]>,
{
    alt((
        map(quoted_string_unchanged, FieldValueFragment::Quoted),
        map(encoded_word, FieldValueFragment::EncodedWord),
        map(field_value_folded_sep, FieldValueFragment::FoldedSep),
        map(encoded_word_space, FieldValueFragment::EncodedWordSpace),
        map(field_value_literal, FieldValueFragment::Literal),
    ))(input)
}

fn field_value<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], ModifiedInput<'a>, E>
where
    E: ParseError<&'a [u8]>,
{
    let remain_begin = input.len();

    let build_string = fold_many0(field_value_body, Vec::new, |mut buf, fragment| {
        match fragment {
            FieldValueFragment::Literal(v) => {
                buf.extend_from_slice(v);
            }
            FieldValueFragment::FoldedSep(_v) => {
                buf.push(b' ');
            }
            FieldValueFragment::Quoted(v) => {
                buf.extend_from_slice(v.0);
                buf.extend_from_slice(v.1);
                buf.extend_from_slice(v.2);
            }
            FieldValueFragment::EncodedWord(v) => {
                buf.extend_from_slice(&v);
            }
            FieldValueFragment::EncodedWordSpace(_v) => {}
        }
        buf
    });

    match terminated(build_string, line_ending)(input) {
        Ok((remain, output)) => {
            let remain_end = remain.len();
            let consumed_len = remain_begin - remain_end;

            Ok((
                remain,
                ModifiedInput {
                    original: &input[..consumed_len],
                    modified: output,
                },
            ))
        }
        Err(error) => Err(error),
    }
}

fn field_pair<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], FieldPair, E>
where
    E: ParseError<&'a [u8]>,
{
    map(
        separated_pair(field_name, tag(b":"), field_value),
        |items| {
            let name = transform_to_string(items.0);
            let name_raw = items.0;
            let field_name = FieldName::new(name, Some(name_raw.to_vec()));

            let value = transform_to_string(&items.1.modified);
            let value_raw = items.1.original;
            let field_value = FieldValue::new(value, Some(value_raw.to_vec()));

            FieldPair::new(field_name, field_value)
        },
    )(input)
}

fn field_pairs<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], Vec<FieldPair>, E>
where
    E: ParseError<&'a [u8]>,
{
    all_consuming(many0(field_pair))(input)
}

pub fn parse_fields(input: &[u8]) -> Result<HeaderMap, nom::Err<VerboseError<&[u8]>>> {
    let output = field_pairs::<VerboseError<&[u8]>>(input)?;
    let pairs = output.1;
    let headers = HeaderMap { pairs };

    Ok(headers)
}

fn transform_to_string(input: &[u8]) -> String {
    let text = String::from_utf8_lossless(input);
    trim(text)
}

fn trim(text: String) -> String {
    let trimmed = text.trim();

    if trimmed.len() != text.len() {
        trimmed.to_string()
    } else {
        text
    }
}

pub fn parse_quoted_string(input: &[u8]) -> Result<String, nom::Err<VerboseError<&[u8]>>> {
    let output = quoted_string::<VerboseError<&[u8]>>(input)?;
    Ok(String::from_utf8_lossless(&output.1))
}

pub fn parse_parameter(input: &[u8]) -> Result<(String, String), nom::Err<VerboseError<&[u8]>>> {
    let output = parameter::<VerboseError<&[u8]>>(input)?;
    let pair = output.1;
    Ok((
        transform_to_string(pair.0),
        String::from_utf8_lossless(&pair.1),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_header() {
        let data = b"k1: v1\r\n\
            k2: v2\r\n";
        let result = parse_fields(data);
        let headers = result.unwrap();

        assert_eq!(headers.len(), 2);
        assert_eq!(headers.get_str("k1"), Some("v1"));
        assert_eq!(headers.get_str("k2"), Some("v2"));
    }

    #[test]
    fn test_empty_value_header() {
        let data = b"k1:\r\n";
        let result = parse_fields(data);
        let headers = result.unwrap();

        assert_eq!(headers.len(), 1);
        assert_eq!(headers.get_str("k1"), Some(""));
    }

    #[test]
    fn test_folded_header() {
        let data = b"k1: Hello\r\n\
            \t \tworld!\r\n\
            k2: v2\r\n";
        let result = parse_fields(data);
        let headers = result.unwrap();

        assert_eq!(headers.len(), 2);
        assert_eq!(headers.get_str("k1"), Some("Hello world!"));
        assert_eq!(headers.get_str("k2"), Some("v2"));
    }

    #[test]
    fn test_quoted_string_header() {
        let data = b"k1: p1=\"v1, \"\r\n";
        let result = parse_fields(data);
        let headers = result.unwrap();

        assert_eq!(headers.get_str("k1"), Some("p1=\"v1, \""));
    }

    #[test]
    fn test_encoded_word_header() {
        let data = b"k1: [=?ISO-8859-1?Q?a?= / =?ISO-8859-1?Q?a?= =?ISO-8859-1?Q?a?=]\r\n";
        let result = parse_fields(data);
        let headers = result.unwrap();

        assert_eq!(headers.get_str("k1"), Some("[a / aa]"));
    }

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

        let data = b"k1=\"hello world!\"";
        let result = parse_parameter(data).unwrap();
        assert_eq!(result.0, "k1");
        assert_eq!(result.1, "hello world!");
    }
}
