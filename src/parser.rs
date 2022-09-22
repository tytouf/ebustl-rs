use std::str::FromStr;

use nom::{
    self,
    bytes::streaming::take,
    combinator::map_res,
    error::{ErrorKind, FromExternalError},
    multi::many1,
    number::streaming::{be_u8, le_u16},
    sequence::tuple,
    InputIter, InputTake,
};
use thiserror::Error;

use super::*;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("IoError: {0}")]
    IoError(String),
    #[error("Error parsing, file may be incomplete or corrupted")]
    Incomplete,
    #[error("Unknown Code Page Number: {0}")]
    CodePageNumber(u16),
    #[error("Error parsing Display Standard Code")]
    DisplayStandardCode,
    #[error("Error parsing Time Code Status")]
    TimeCodeStatus,
    #[error("Error parsing Disk Format Code: {0}")]
    DiskFormatCode(String),
    #[error("Error parsing Character Code Table")]
    CharacterCodeTable,
    #[error("Error parsing Cumulative Status")]
    CumulativeStatus,
    #[error("Parse error: {message}")]
    NomParsingError { message: String },
    #[error("Unknown error")]
    Unknown,
}

impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err.to_string())
    }
}

impl<I> nom::error::ParseError<I> for ParseError {
    // on one line, we show the error code and the input that caused it
    fn from_error_kind(_: I, kind: ErrorKind) -> Self {
        Self::NomParsingError {
            message: format!("{:?}", kind),
        }
    }

    // if combining multiple errors, we show them one after the other
    fn append(_: I, kind: ErrorKind, other: Self) -> Self {
        let message = format!("{:?}:\n{}", kind, other);
        Self::NomParsingError { message }
    }

    fn from_char(_: I, c: char) -> Self {
        Self::NomParsingError {
            message: format!("unexpected character '{}'", c),
        }
    }
}

impl<I, E> FromExternalError<I, E> for ParseError
where
    E: fmt::Display,
{
    fn from_external_error(_: I, kind: ErrorKind, e: E) -> Self {
        let message = format!("{:?}:\n{}", kind, e);
        Self::NomParsingError { message }
    }
}

impl<E> From<nom::Err<E>> for ParseError
where
    E: fmt::Display,
{
    fn from(err: nom::Err<E>) -> Self {
        match err {
            nom::Err::Incomplete(_) => ParseError::Incomplete,
            nom::Err::Error(e) | nom::Err::Failure(e) => Self::NomParsingError {
                message: format!("{}", e),
            },
        }
    }
}

pub type IResult<I, O> = nom::IResult<I, O, ParseError>;

fn parse_stl(input: &[u8]) -> IResult<&[u8], Stl> {
    let (input, (gsi, ttis)) = tuple((parse_gsi_block, many1(parse_tti_block)))(input)?;
    Ok((input, Stl { gsi, ttis }))
}

pub fn parse_stl_from_slice(input: &[u8]) -> Result<Stl, ParseError> {
    let (_, stl) = parse_stl(input)?;
    Ok(stl)
}

pub fn take_str<'a, C: nom::ToUsize, Error: nom::error::ParseError<&'a [u8]>>(
    count: C,
) -> impl Fn(&'a [u8]) -> nom::IResult<&'a [u8], &'a str, Error> {
    let c = count.to_usize();
    move |i: &[u8]| match i.slice_index(c) {
        Err(i) => Err(nom::Err::Incomplete(i)),
        Ok(index) => {
            let (first, rest) = i.take_split(index);
            Ok((
                first,
                str::from_utf8(rest).map_err(|_err| {
                    nom::Err::Error(Error::from_error_kind(rest, nom::error::ErrorKind::Fail))
                })?,
            ))
        }
    }
}

fn parse_gsi_block(input: &[u8]) -> IResult<&[u8], GsiBlock> {
    let (input, (codepage, dfc, dsc, cct)) = tuple((
        map_res(take_str(3_u16), u16::from_str),
        map_res(take_str(10 - 3 + 1_u16), DiskFormatCode::parse),
        map_res(be_u8, DisplayStandardCode::parse),
        map_res(take(13 - 12 + 1_u16), CharacterCodeTable::parse),
    ))(input)?;
    let cpn = CodePageNumber::from_u16(codepage).map_err(nom::Err::Error)?;
    let coding = CodePageDecoder::new(codepage).map_err(nom::Err::Error)?;

    let (input, (lc, opt, oet, tpt, tet, tn, tcd, slr, cd, rd, rn, tnb, tns, tng, mnc, mnr, tcs)) =
        tuple((
            map_res(take(15 - 14 + 1_u16), |data| coding.parse(data)),
            map_res(take(47 - 16 + 1_u16), |data| coding.parse(data)),
            map_res(take(79 - 48 + 1_u16), |data| coding.parse(data)),
            map_res(take(111 - 80 + 1_u16), |data| coding.parse(data)),
            map_res(take(143 - 112 + 1_u16), |data| coding.parse(data)),
            map_res(take(175 - 144 + 1_u16), |data| coding.parse(data)),
            map_res(take(207 - 176 + 1_u16), |data| coding.parse(data)),
            map_res(take(223 - 208 + 1_u16), |data| coding.parse(data)),
            map_res(take(229 - 224 + 1_u16), |data| coding.parse(data)),
            map_res(take(235 - 230 + 1_u16), |data| coding.parse(data)),
            map_res(take(237 - 236 + 1_u16), |data| coding.parse(data)),
            map_res(take_str(242 - 238 + 1_u16), u16::from_str),
            map_res(take_str(247 - 243 + 1_u16), u16::from_str),
            map_res(take_str(250 - 248 + 1_u16), u16::from_str),
            map_res(take_str(252 - 251 + 1_u16), u16::from_str),
            map_res(take_str(254 - 253 + 1_u16), u16::from_str),
            map_res(be_u8, TimeCodeStatus::parse),
        ))(input)?;

    let (input, (tcp, tcf, tnd, dsn, co, pub_, en, ecd, _spare, uda)) = tuple((
        map_res(take(263 - 256 + 1_u16), |data| coding.parse(data)),
        map_res(take(271 - 264 + 1_u16), |data| coding.parse(data)),
        map_res(take_str(1_u16), u8::from_str),
        map_res(take_str(1_u16), u8::from_str),
        map_res(take(276 - 274 + 1_u16), |data| coding.parse(data)),
        map_res(take(308 - 277 + 1_u16), |data| coding.parse(data)),
        map_res(take(340 - 309 + 1_u16), |data| coding.parse(data)),
        map_res(take(372 - 341 + 1_u16), |data| coding.parse(data)),
        map_res(take(447 - 373 + 1_u16), |data| coding.parse(data)),
        map_res(take(1023 - 448 + 1_u16), |data| coding.parse(data)),
    ))(input)?;
    Ok((
        input,
        GsiBlock {
            cpn,
            dfc,
            dsc,
            cct,
            lc,
            opt,
            oet,
            tpt,
            tet,
            tn,
            tcd,
            slr,
            cd,
            rd,
            rn,
            tnb,
            tns,
            tng,
            mnc,
            mnr,
            tcs,
            tcp,
            tcf,
            tnd,
            dsn,
            co,
            pub_,
            en,
            ecd,
            _spare,
            uda,
        },
    ))
}

fn parse_time(input: &[u8]) -> IResult<&[u8], Time> {
    let (input, (h, m, s, f)) = tuple((be_u8, be_u8, be_u8, be_u8))(input)?;
    Ok((input, Time::new(h, m, s, f)))
}

fn parse_tti_block(input: &[u8]) -> IResult<&[u8], TtiBlock> {
    //Needed to handle the many1 operator, that expects an error when done.
    if input.is_empty() {
        return Err(nom::Err::Error(nom::error::ParseError::from_error_kind(
            input,
            nom::error::ErrorKind::Eof,
        )));
    }
    let (input, (sgn, sn, ebn, cs, tci, tco, vp, jc, cf, tf)) = tuple((
        be_u8,
        le_u16,
        be_u8,
        map_res(be_u8, CumulativeStatus::parse),
        parse_time,
        parse_time,
        be_u8,
        be_u8,
        be_u8,
        take(112_u16),
    ))(input)?;
    Ok((
        input,
        TtiBlock {
            sgn,
            sn,
            ebn,
            cs,
            tci,
            tco,
            vp,
            jc,
            cf,
            tf: tf.to_vec(),
        },
    ))
}

#[cfg(test)]
mod tests {
    use nom::Needed;

    use super::*;

    #[test]
    fn test_parse_time() {
        let empty: &[u8] = &vec![];
        let ok = &vec![0x1, 0x2, 0x3, 0x4];
        let incomplete = &vec![0x1];

        assert_eq!(
            parse_time(ok),
            Ok((
                empty,
                Time {
                    hours: 1,
                    minutes: 2,
                    seconds: 3,
                    frames: 4,
                }
            ))
        );
        assert_eq!(
            parse_time(incomplete),
            Err(nom::Err::Incomplete(Needed::new(1)))
        );
    }
    //Comented out since the test file is propritary
    #[test]
    fn test_parse_file() {
        let stl = parse_stl_from_file("stls/test.stl")
            .map_err(|err| {
                eprintln!("Error: {}", err);
                err.to_string()
            })
            .expect("Parse stl");
        println!("STL:\n{:?}", stl);
        assert_eq!(1_u8, stl.gsi.tnd);
        assert_eq!(1_u8, stl.gsi.dsn);
        assert_eq!(13, stl.ttis.len());
        assert_eq!(
            "    dans la baie de New York.\r\n",
            stl.ttis.get(11).unwrap().get_text()
        );
    }
    /* TODO
    #[test]
    fn test_parse_tti() {
    }
    fn test_parse_gsi() {
    }
    */
}
