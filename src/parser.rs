use std::str::FromStr;

use nom::{
    self,
    bytes::streaming::take,
    combinator::map_res,
    multi::many1,
    number::streaming::{be_u8, le_u16},
    sequence::tuple,
    IResult, InputIter, InputTake,
};
use thiserror::Error;

use super::*;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("Error parsing, file may be incomplete or corrupted")]
    Incomplete,
    #[error("Error parsing Code Page Number")]
    CodePageNumber,
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
    #[error("Unknown error")]
    Unknown,
}

fn parse_stl(input: &[u8]) -> IResult<&[u8], Stl> {
    let (input, (gsi, ttis)) = tuple((parse_gsi_block, many1(parse_tti_block)))(input)?;
    Ok((input, Stl { gsi, ttis }))
}

pub fn parse_stl_from_slice(input: &[u8]) -> Result<Stl, ParseError> {
    match parse_stl(input) {
        Ok((_, stl)) => Ok(stl),
        Err(nom::Err::Error(_) | nom::Err::Failure(_)) => Err(ParseError::Unknown),
        Err(nom::Err::Incomplete(_)) => Err(ParseError::Incomplete),
    }
}

pub fn take_str<'a, C: nom::ToUsize, Error: nom::error::ParseError<&'a [u8]>>(
    count: C,
) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], &'a str, Error> {
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
    let cpn = CodePageNumber::from_u16(codepage).map_err(|_e| {
        nom::Err::Error(nom::error::ParseError::from_error_kind(
            input,
            nom::error::ErrorKind::Fail,
        ))
    })?;
    let coding = CodePageDecoder::new(codepage).map_err(|_e| {
        nom::Err::Error(nom::error::ParseError::from_error_kind(
            input,
            nom::error::ErrorKind::Fail,
        ))
    })?;

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
            .map_err(|err| err.to_string())
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
