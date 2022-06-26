use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take, take_until},
    character::complete::{line_ending, space1},
    combinator::{all_consuming, map},
    error::{ParseError, VerboseError},
    multi::{fold_many0, many0},
    sequence::{delimited, pair, separated_pair, terminated, tuple},
    FindSubstring, IResult,
};

use super::{FieldName, FieldPair, FieldValue, HeaderMap};

struct ModifiedInput<'a> {
    original: &'a [u8],
    modified: Vec<u8>,
}

fn quoted_string_text_unchanged<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], &'a [u8], E>
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
    tuple((tag(b"\""), quoted_string_text_unchanged, tag(b"\"")))(input)
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

fn field_value_text<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], FieldValueFragment, E>
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

    let result = fold_many0(field_value_text, Vec::new, |mut buf, fragment| {
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
    })(input);

    match result {
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
            let name = crate::stringutil::decode_and_trim_to_string(items.0);
            let name_raw = items.0;
            let field_name = FieldName::new(name, Some(name_raw.to_vec()));

            let value = crate::stringutil::decode_and_trim_to_string(&items.1.modified);
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
    all_consuming(many0(terminated(field_pair, line_ending)))(input)
}

pub fn parse_fields(input: &[u8]) -> Result<HeaderMap, nom::Err<VerboseError<&[u8]>>> {
    let output = field_pairs::<VerboseError<&[u8]>>(input)?;
    let pairs = output.1;
    let headers = HeaderMap { pairs };

    Ok(headers)
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
}
