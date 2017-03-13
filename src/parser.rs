use nom::{IResult, be_u8, le_u16};
use std::error;
use std::error::Error;
use std::io;
use std::str;
use std::str::FromStr;

use super::*;


#[derive(Debug)]
pub enum ParseError {
    Io(io::Error),
    Incomplete,
    CodePageNumber,
    DisplayStandardCode,
    TimeCodeStatus,
    DiskFormatCode,
    CharacterCodeTable,
    CumulativeStatus,
    Unknown,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ParseError: {}", self.description())
    }
}

impl error::Error for ParseError {
    fn description(&self) -> &str {
        match *self {
            ParseError::Io(ref err) => err.description(),
            ParseError::Incomplete => "Error parsing, file may be incomplete or corrupted",
            ParseError::CodePageNumber => "Error parsing Code Page Number",
            ParseError::DisplayStandardCode => "Error parsing Display Standard Code",
            ParseError::TimeCodeStatus => "Error parsing Time Code Status",
            ParseError::DiskFormatCode => "Error parsing Disk Format Code",
            ParseError::CharacterCodeTable => "Error parsing Character Code Table",
            ParseError::CumulativeStatus => "Error parsing Cumulative Status",
            ParseError::Unknown => "Unknown error",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

impl From<io::Error> for ParseError {
    fn from(err: io::Error) -> ParseError {
        ParseError::Io(err)
    }
}

named!(parse_stl<Stl>,
  do_parse!(
    gsi: parse_gsi_block >>
    ttis: many1!(parse_tti_block) >>
    (Stl{gsi: gsi, ttis: ttis})
  )
);

pub fn parse_stl_from_slice(input: &[u8]) -> Result<Stl, ParseError> {
    match parse_stl(input) {
        IResult::Error(_) => Err(ParseError::Unknown), //TODO from custom to ParseError
        IResult::Incomplete(_) => Err(ParseError::Incomplete),
        IResult::Done(_, stl) => Ok(stl),
    }
}

named!(parse_gsi_block<GsiBlock>,
  do_parse!(
    cpn: map_res!(take!(3), CodePageNumber::parse)       >>
    dfc: map_res!(take_str!(10-3+1), DiskFormatCode::parse)    >>
    dsc: map_res!(be_u8, DisplayStandardCode::parse)       >>
    cct: map_res!(take!(13-12+1), CharacterCodeTable::parse)   >>
    lc: map_res!(take_str!(15-14+1), String::from_str)   >>
    opt: map_res!(take_str!(47-16+1), String::from_str)  >>
    oet: map_res!(take_str!(79-48+1), String::from_str)  >>
    tpt: map_res!(take_str!(111-80+1), String::from_str)  >>
    tet: map_res!(take_str!(143-112+1), String::from_str)  >>
    tn: map_res!(take_str!(175-144+1), String::from_str)  >>
    tcd: map_res!(take_str!(207-176+1), String::from_str)  >>
    slr: map_res!(take_str!(223-208+1), String::from_str)  >>
    cd: map_res!(take_str!(229-224+1), String::from_str)  >>
    rd: map_res!(take_str!(235-230+1), String::from_str)  >>
    rn: map_res!(take_str!(237-236+1), String::from_str)  >>
    tnb: map_res!(take_str!(242-238+1), u16::from_str)   >>
    tns: map_res!(take_str!(247-243+1), u16::from_str)   >>
    tng: map_res!(take_str!(250-248+1), u16::from_str)   >>
    mnc: map_res!(take_str!(252-251+1), u16::from_str)   >>
    mnr: map_res!(take_str!(254-253+1), u16::from_str)   >>
    tcs: map_res!(be_u8, TimeCodeStatus::parse)   >>
    tcp: map_res!(take_str!(263-256+1), String::from_str)  >>
    tcf: map_res!(take_str!(271-264+1), String::from_str)  >>
    tnd: map_res!(take_str!(1), u8::from_str)   >>
    dsn: map_res!(take_str!(1), u8::from_str)   >>
    co: map_res!(take_str!(276-274+1), String::from_str)  >>
    pub_: map_res!(take_str!(308-277+1), String::from_str)  >>
    en: map_res!(take_str!(340-309+1), String::from_str)  >>
    ecd: map_res!(take_str!(372-341+1), String::from_str)  >>
    _spare: map_res!(take_str!(447-373+1), String::from_str)  >>
    uda: map_res!(take_str!(1023-448+1), String::from_str)  >>
    (GsiBlock{
        cpn: cpn, dfc: dfc, dsc: dsc, cct: cct, lc: lc,
        opt: opt, oet: oet, tpt: tpt, tet: tet, tn: tn, tcd: tcd, slr: slr,
        cd: cd, rd: rd, rn: rn, tnb: tnb, tns: tns, tng: tng,
        mnc: mnc, mnr: mnr, tcs: tcs, tcp: tcp, tcf: tcf, tnd: tnd, dsn: dsn,
        co: co, pub_: pub_, en: en, ecd: ecd, _spare: _spare, uda: uda,
        })
  )
);

named!(parse_time<Time>,
  do_parse!(
    h: be_u8 >>
    m: be_u8 >>
    s: be_u8 >>
    f: be_u8 >>
    (Time::new(h, m, s, f))
  )
);

named!(parse_tti_block<TtiBlock>,
  do_parse!(
	sgn: be_u8 >>
    sn: le_u16 >>
    ebn: be_u8 >>
    cs: map_res!(be_u8, CumulativeStatus::parse) >>
    tci: parse_time >>
    tco: parse_time >>
    vp: be_u8 >>
    jc: be_u8 >>
    cf: be_u8 >>
    tf: take!(112) >>
    (TtiBlock{sgn: sgn, sn: sn, ebn: ebn, cs: cs, tci: tci, tco: tco, vp: vp,
        jc: jc, cf: cf, tf: tf.to_vec()})
  )
);

#[cfg(test)]
mod tests {
    use super::*;
    use nom::IResult::*;
    use nom::Needed;

    #[test]
    fn test_parse_time() {
        let empty: &[u8] = &vec![];
        let ok = &vec![0x1, 0x2, 0x3, 0x4];
        let incomplete = &vec![0x1];

        assert_eq!(parse_time(ok), Done(empty, Time{
            hours: 1,
            minutes: 2,
            seconds: 3,
            frames: 4,
            }));
        assert_eq!(parse_time(incomplete), Incomplete(Needed::Size(incomplete.len()+1)));
    }

    /* TODO
    #[test]
    fn test_parse_tti() {
    }
    fn test_parse_gsi() {
    }
    */
}
