extern crate chrono;
extern crate nom;

use std::fmt;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::str;

use codepage_strings::Coding;
use textcode::{iso6937, iso8859_5, iso8859_6, iso8859_7, iso8859_8};
pub mod parser;
use crate::parser::parse_stl_from_slice;
pub use crate::parser::ParseError;

// STL File

#[derive(Debug)]
pub struct Stl {
    pub gsi: GsiBlock,
    pub ttis: Vec<TtiBlock>,
}

impl fmt::Display for Stl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\n{:?}\n", self.gsi, self.ttis)
    }
}

pub struct TtiFormat {
    #[doc = "Justification Code"]
    pub jc: u8,
    #[doc = "Vertical Position"]
    pub vp: u8,
    #[doc = "Double Height"]
    pub dh: bool,
}

impl Stl {
    pub fn new() -> Stl {
        Stl {
            gsi: GsiBlock::new(),
            ttis: vec![],
        }
    }

    pub fn write_to_file(&self, filename: &str) -> Result<(), io::Error> {
        let mut f = File::create(filename)?;
        f.write_all(&self.gsi.serialize())?;
        for tti in self.ttis.iter() {
            f.write_all(&tti.serialize())?;
        }
        Ok(())
    }

    pub fn add_sub(&mut self, tci: Time, tco: Time, txt: &str, opt: TtiFormat) {
        if txt.len() > 112 {
            //TODO: if txt.len() > 112 split in multiple
            println!("Warning: sub text is too long!");
        }
        self.gsi.tnb += 1; // First TTI has sn=1
        let tti = TtiBlock::new(self.gsi.tnb, tci, tco, txt, opt, self.gsi.cct);
        self.gsi.tns += 1;
        self.ttis.push(tti);
    }
}

impl Default for Stl {
    fn default() -> Self {
        Self::new()
    }
}

pub fn parse_stl_from_file(filename: &str) -> Result<Stl, ParseError> {
    let mut f = File::open(filename)?;
    let mut buffer = vec![];
    f.read_to_end(&mut buffer)?;

    parse_stl_from_slice(&buffer)
}

struct CodePageDecoder {
    coding: Coding,
}

impl CodePageDecoder {
    pub fn new(codepage: u16) -> Result<Self, ParseError> {
        Ok(Self {
            coding: Coding::new(codepage).map_err(|_e| ParseError::CodePageNumber(codepage))?,
        })
    }

    fn parse(&self, data: &[u8]) -> Result<String, ParseError> {
        Ok(self.coding.decode_lossy(data).to_string())
    }
}

// GSI Block

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum CodePageNumber {
    CPN_437,
    CPN_850,
    CPN_860,
    CPN_863,
    CPN_865,
}

impl CodePageNumber {
    fn serialize(&self) -> Vec<u8> {
        match *self {
            CodePageNumber::CPN_437 => vec![0x34, 0x33, 0x37],
            CodePageNumber::CPN_850 => vec![0x38, 0x35, 0x30],
            CodePageNumber::CPN_860 => vec![0x38, 0x36, 0x30],
            CodePageNumber::CPN_863 => vec![0x38, 0x36, 0x33],
            CodePageNumber::CPN_865 => vec![0x38, 0x36, 0x35],
        }
    }

    pub(crate) fn from_u16(codepage: u16) -> Result<CodePageNumber, ParseError> {
        match codepage {
            437 => Ok(CodePageNumber::CPN_437),
            850 => Ok(CodePageNumber::CPN_850),
            860 => Ok(CodePageNumber::CPN_860),
            863 => Ok(CodePageNumber::CPN_863),
            865 => Ok(CodePageNumber::CPN_865),
            _ => Err(ParseError::CodePageNumber(codepage)),
        }
    }
}

#[derive(Debug)]
pub enum DisplayStandardCode {
    Blank,
    OpenSubtitling,
    Level1Teletext,
    Level2Teletext,
}

impl DisplayStandardCode {
    fn parse(data: u8) -> Result<DisplayStandardCode, ParseError> {
        match data {
            0x20 => Ok(DisplayStandardCode::Blank),
            0x30 => Ok(DisplayStandardCode::OpenSubtitling),
            0x31 => Ok(DisplayStandardCode::Level1Teletext),
            0x32 => Ok(DisplayStandardCode::Level2Teletext),
            _ => Err(ParseError::DisplayStandardCode),
        }
    }

    fn serialize(&self) -> u8 {
        match *self {
            DisplayStandardCode::Blank => 0x20,
            DisplayStandardCode::OpenSubtitling => 0x30,
            DisplayStandardCode::Level1Teletext => 0x31,
            DisplayStandardCode::Level2Teletext => 0x32,
        }
    }
}

#[derive(Debug)]
pub enum TimeCodeStatus {
    NotIntendedForUse,
    IntendedForUse,
}

impl TimeCodeStatus {
    fn parse(data: u8) -> Result<TimeCodeStatus, ParseError> {
        match data {
            0x30 => Ok(TimeCodeStatus::NotIntendedForUse),
            0x31 => Ok(TimeCodeStatus::IntendedForUse),
            _ => Err(ParseError::TimeCodeStatus),
        }
    }

    fn serialize(&self) -> u8 {
        match *self {
            TimeCodeStatus::NotIntendedForUse => 0x30,
            TimeCodeStatus::IntendedForUse => 0x31,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum CharacterCodeTable {
    Latin,
    LatinCyrillic,
    LatinArabic,
    LatinGreek,
    LatinHebrew,
}

impl CharacterCodeTable {
    fn parse(data: &[u8]) -> Result<CharacterCodeTable, ParseError> {
        if data.len() != 2 {
            return Err(ParseError::CharacterCodeTable);
        }
        if data[0] != 0x30 {
            return Err(ParseError::CharacterCodeTable);
        }
        match data[1] {
            0x30 => Ok(CharacterCodeTable::Latin),
            0x31 => Ok(CharacterCodeTable::LatinCyrillic),
            0x32 => Ok(CharacterCodeTable::LatinArabic),
            0x33 => Ok(CharacterCodeTable::LatinGreek),
            0x34 => Ok(CharacterCodeTable::LatinHebrew),
            _ => Err(ParseError::CharacterCodeTable),
        }
    }

    fn serialize(&self) -> Vec<u8> {
        match *self {
            CharacterCodeTable::Latin => vec![0x30, 0x30],
            CharacterCodeTable::LatinCyrillic => vec![0x30, 0x31],
            CharacterCodeTable::LatinArabic => vec![0x30, 0x32],
            CharacterCodeTable::LatinGreek => vec![0x30, 0x33],
            CharacterCodeTable::LatinHebrew => vec![0x30, 0x34],
        }
    }
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum DiskFormatCode {
    STL25_01,
    STL30_01,
}

impl DiskFormatCode {
    fn parse(data: &str) -> Result<DiskFormatCode, ParseError> {
        if data == "STL25.01" {
            Ok(DiskFormatCode::STL25_01)
        } else if data == "STL30.01" {
            Ok(DiskFormatCode::STL30_01)
        } else {
            Err(ParseError::DiskFormatCode(data.to_string()))
        }
    }

    fn serialize(&self) -> Vec<u8> {
        match *self {
            DiskFormatCode::STL25_01 => String::from("STL25.01").into_bytes(),
            DiskFormatCode::STL30_01 => String::from("STL30.01").into_bytes(),
        }
    }

    pub fn get_fps(&self) -> usize {
        match self {
            DiskFormatCode::STL25_01 => 25,
            DiskFormatCode::STL30_01 => 30,
        }
    }
}

#[derive(Debug)]
pub struct GsiBlock {
    #[doc = "0..2 Code Page Number"]
    cpn: CodePageNumber,
    #[doc = "3..10 Disk Format Code"]
    dfc: DiskFormatCode,
    #[doc = "11 Display Standard Code"]
    dsc: DisplayStandardCode,
    #[doc = "12..13 Character Code Table Number"]
    cct: CharacterCodeTable,
    #[doc = "14..15 Language Code"]
    lc: String,
    #[doc = "16..47 Original Program Title"]
    opt: String,
    #[doc = "48..79 Original Episode Title"]
    oet: String,
    #[doc = "80..111 Translated Program Title"]
    tpt: String,
    #[doc = "112..143 Translated Episode Title"]
    tet: String,
    #[doc = "144..175 Translator's Name"]
    tn: String,
    #[doc = "176..207 Translator's Contact Details"]
    tcd: String,
    #[doc = "208..223 Subtitle List Reference Code"]
    slr: String,
    #[doc = "224..229 Creation Date"]
    cd: String,
    #[doc = "230..235 Revision Date"]
    rd: String,
    #[doc = "236..237 Revision Number"]
    rn: String,
    #[doc = "238..242 Total Number of Text and Timing Blocks"]
    tnb: u16,
    #[doc = "243..247 Total Number of Subtitles"]
    tns: u16,
    #[doc = "248..250 Total Number of Subtitle Groups"]
    tng: u16,
    #[doc = "251..252 Maximum Number of Displayable Characters in a Text Row"]
    mnc: u16,
    #[doc = "253..254 Maximum Number of Displayable Rows"]
    mnr: u16,
    #[doc = "255 Time Code Status"]
    tcs: TimeCodeStatus,
    #[doc = "256..263 Time Code: Start of Programme (format: HHMMSSFF)"]
    tcp: String,
    #[doc = "264..271 Time Code: First-in-Cue (format: HHMMSSFF)"]
    tcf: String,
    #[doc = "272 Total Number of Disks"]
    tnd: u8,
    #[doc = "273 Disk Sequence Number"]
    dsn: u8,
    #[doc = "274..276 Country of Origin"]
    co: String, // TODO Type with country definitions
    #[doc = "277..308 Publisher"]
    pub_: String,
    #[doc = "309..340 Editor's Name"]
    en: String,
    #[doc = "341..372 Editor's Contact Details"]
    ecd: String,
    #[doc = "373..447 Spare Bytes"]
    _spare: String,
    #[doc = "448..1023 User-Defined Area"]
    uda: String,
}

impl GsiBlock {
    pub fn get_code_page_number(&self) -> &CodePageNumber {
        &self.cpn
    }
    pub fn get_disk_format_code(&self) -> &DiskFormatCode {
        &self.dfc
    }
    pub fn get_display_standard_code(&self) -> &DisplayStandardCode {
        &self.dsc
    }
    pub fn get_character_code_table(&self) -> &CharacterCodeTable {
        &self.cct
    }
    pub fn get_language_code(&self) -> &str {
        &self.lc
    }
    pub fn get_original_program_title(&self) -> &str {
        &self.opt
    }
    pub fn get_original_episode_title(&self) -> &str {
        &self.oet
    }
    pub fn get_translated_program_title(&self) -> &str {
        &self.tpt
    }
    pub fn get_translated_episode_title(&self) -> &str {
        &self.tet
    }
    pub fn get_translators_name(&self) -> &str {
        &self.tn
    }
    pub fn get_translators_contact_details(&self) -> &str {
        &self.tcd
    }
    pub fn get_subtitle_list_reference_code(&self) -> &str {
        &self.slr
    }
    pub fn get_creation_date(&self) -> &str {
        &self.cd
    }
    pub fn get_revision_date(&self) -> &str {
        &self.rd
    }
    pub fn get_revision_number(&self) -> &str {
        &self.rn
    }
    pub fn get_total_number_of_text_and_timing_blocks(&self) -> u16 {
        self.tnb
    }
    pub fn get_total_number_of_subtitles(&self) -> u16 {
        self.tns
    }
    pub fn get_total_number_of_chars_in_row(&self) -> u16 {
        self.tng
    }
    pub fn get_max_number_of_chars_in_row(&self) -> u16 {
        self.mnc
    }
    pub fn get_max_number_of_rows(&self) -> u16 {
        self.mnr
    }
    pub fn get_timecode_status(&self) -> &TimeCodeStatus {
        &self.tcs
    }
    pub fn get_timecode_start_of_program(&self) -> &str {
        &self.tcp
    }
    pub fn get_timecode_first_in_cue(&self) -> &str {
        &self.tcf
    }
    pub fn get_total_number_of_disks(&self) -> u8 {
        self.tnd
    }
    pub fn get_disk_sequence_number(&self) -> u8 {
        self.dsn
    }
    pub fn get_country_of_origin(&self) -> &str {
        &self.co
    }
    pub fn get_publisher(&self) -> &str {
        &self.pub_
    }
    pub fn get_editors_name(&self) -> &str {
        &self.en
    }
    pub fn get_editors_contact_details(&self) -> &str {
        &self.ecd
    }
    pub fn get_user_defined_area(&self) -> &str {
        &self.uda
    }
}

fn push_string(v: &mut Vec<u8>, s: &str, len: usize) {
    let addendum = s.to_owned().into_bytes();
    let padding = len - addendum.len();
    v.extend(addendum.iter().cloned());
    v.extend(vec![0x20u8; padding]);
}

impl GsiBlock {
    pub fn new() -> GsiBlock {
        let date = chrono::Local::now();
        let now = date.format("%y%m%d").to_string();
        GsiBlock {
            cpn: CodePageNumber::CPN_850,
            dfc: DiskFormatCode::STL25_01,
            dsc: DisplayStandardCode::Level1Teletext,
            cct: CharacterCodeTable::Latin,
            lc: "0F".to_string(), // FIXME: ok for default?
            opt: "".to_string(),
            oet: "".to_string(),
            tpt: "".to_string(),
            tet: "".to_string(),
            tn: "".to_string(),
            tcd: "".to_string(),
            slr: "".to_string(),
            cd: now.clone(),
            rd: now,
            rn: "00".to_string(),
            tnb: 0,
            tns: 0,
            tng: 1,  // At least one group?
            mnc: 40, // FIXME: ok for default?
            mnr: 23, // FIXME: ok for default?
            tcs: TimeCodeStatus::IntendedForUse,
            tcp: "00000000".to_string(),
            tcf: "00000000".to_string(),
            tnd: 1,
            dsn: 1,
            co: "".to_string(),
            pub_: "".to_string(),
            en: "".to_string(),
            ecd: "".to_string(),
            _spare: "".to_string(),
            uda: "".to_string(),
        }
    }

    fn serialize(&self) -> Vec<u8> {
        let mut res = Vec::with_capacity(1024);
        res.extend(self.cpn.serialize());
        res.extend(self.dfc.serialize().iter().cloned());
        res.push(self.dsc.serialize());
        res.extend(self.cct.serialize());
        // be careful for the length of following: must force padding
        push_string(&mut res, &self.lc, 15 - 14 + 1);
        push_string(&mut res, &self.opt, 47 - 16 + 1);
        push_string(&mut res, &self.oet, 79 - 48 + 1);
        push_string(&mut res, &self.tpt, 111 - 80 + 1);
        push_string(&mut res, &self.tet, 143 - 112 + 1);
        push_string(&mut res, &self.tn, 175 - 144 + 1);
        push_string(&mut res, &self.tcd, 207 - 176 + 1);
        push_string(&mut res, &self.slr, 223 - 208 + 1);
        push_string(&mut res, &self.cd, 229 - 224 + 1);
        push_string(&mut res, &self.rd, 235 - 230 + 1);
        push_string(&mut res, &self.rn, 237 - 236 + 1);

        push_string(&mut res, &format!("{:05}", self.tnb), 242 - 238 + 1);
        push_string(&mut res, &format!("{:05}", self.tns), 247 - 243 + 1);
        push_string(&mut res, &format!("{:03}", self.tng), 250 - 248 + 1);
        push_string(&mut res, &format!("{:02}", self.mnc), 252 - 251 + 1);
        push_string(&mut res, &format!("{:02}", self.mnr), 254 - 253 + 1);

        res.push(self.tcs.serialize());
        push_string(&mut res, &self.tcp, 263 - 256 + 1);
        push_string(&mut res, &self.tcf, 271 - 264 + 1);
        push_string(&mut res, &format!("{:1}", self.tnd), 1);
        push_string(&mut res, &format!("{:1}", self.dsn), 1);
        push_string(&mut res, &self.co, 276 - 274 + 1);
        push_string(&mut res, &self.pub_, 308 - 277 + 1);
        push_string(&mut res, &self.en, 340 - 309 + 1);
        push_string(&mut res, &self.ecd, 372 - 341 + 1);
        push_string(&mut res, &self._spare, 447 - 373 + 1);
        push_string(&mut res, &self.uda, 1023 - 448 + 1);

        res
    }
}

impl Default for GsiBlock {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for GsiBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Program Title: {}\nEpisode Title: {}\ncct:{:?} lc:{}\n",
            self.opt, self.oet, self.cct, self.lc
        )
    }
}

// TTI Block

#[derive(Debug)]
pub enum CumulativeStatus {
    NotPartOfASet,
    FirstInSet,
    IntermediateInSet,
    LastInSet,
}

impl CumulativeStatus {
    fn parse(d: u8) -> Result<CumulativeStatus, ParseError> {
        match d {
            0 => Ok(CumulativeStatus::NotPartOfASet),
            1 => Ok(CumulativeStatus::FirstInSet),
            2 => Ok(CumulativeStatus::IntermediateInSet),
            3 => Ok(CumulativeStatus::LastInSet),
            _ => Err(ParseError::CumulativeStatus),
        }
    }

    fn serialize(&self) -> u8 {
        match *self {
            CumulativeStatus::NotPartOfASet => 0,
            CumulativeStatus::FirstInSet => 1,
            CumulativeStatus::IntermediateInSet => 2,
            CumulativeStatus::LastInSet => 3,
        }
    }
}

pub enum Justification {
    Unchanged,
    Left,
    Centered,
    Right,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Time {
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
    pub frames: u8,
}

impl Time {
    fn new(h: u8, m: u8, s: u8, f: u8) -> Time {
        Time {
            hours: h,
            minutes: m,
            seconds: s,
            frames: f,
        }
    }

    pub fn format_fps(&self, fps: usize) -> String {
        format!(
            "{}:{}:{},{}",
            self.hours,
            self.minutes,
            self.seconds,
            self.frames as usize * 1000 / fps
        )
    }
    fn serialize(&self) -> Vec<u8> {
        vec![self.hours, self.minutes, self.seconds, self.frames]
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}/{})",
            self.hours, self.minutes, self.seconds, self.frames
        )
    }
}

pub struct TtiBlock {
    #[doc = "0 Subtitle Group Number. 00h-FFh"]
    sgn: u8,
    #[doc = "1..2 Subtitle Number range. 0000h-FFFFh"]
    sn: u16,
    #[doc = "3 Extension Block Number. 00h-FFh"]
    ebn: u8,
    #[doc = "4 Cumulative Status. 00-03h"]
    cs: CumulativeStatus,
    #[doc = "5..8 Time Code In"]
    tci: Time,
    #[doc = "9..12 Time Code Out"]
    tco: Time,
    #[doc = "13 Vertical Position"]
    vp: u8,
    #[doc = "14 Justification Code"]
    jc: u8,
    #[doc = "15 Comment Flag"]
    cf: u8,
    #[doc = "16..127 Text Field"]
    tf: Vec<u8>,
    #[doc = "Duplication of the CharacterCodeTable in GsiBlock, do be able to decode/encode text independent of the GsiBlock"]
    cct: CharacterCodeTable, //Needed for Display/Debug without access to GsiBlock
}

impl TtiBlock {
    pub fn get_subtitle_group_number(&self) -> u8 {
        self.sgn
    }
    pub fn get_subtitle_number_range(&self) -> u16 {
        self.sn
    }
    pub fn get_extension_block_number(&self) -> u8 {
        self.ebn
    }
    pub fn get_cumulative_status(&self) -> &CumulativeStatus {
        &self.cs
    }
    pub fn get_time_code_in(&self) -> &Time {
        &self.tci
    }
    pub fn get_time_code_out(&self) -> &Time {
        &self.tco
    }
    pub fn get_vertical_position(&self) -> u8 {
        self.vp
    }
    pub fn get_justification_code(&self) -> u8 {
        self.jc
    }
    pub fn get_comment_flag(&self) -> u8 {
        self.cf
    }
}

impl TtiBlock {
    pub fn new(
        idx: u16,
        tci: Time,
        tco: Time,
        txt: &str,
        opt: TtiFormat,
        cct: CharacterCodeTable,
    ) -> TtiBlock {
        TtiBlock {
            sgn: 0,
            sn: idx,
            ebn: 0xff,
            cs: CumulativeStatus::NotPartOfASet,
            tci,
            tco,
            vp: opt.vp,
            jc: opt.jc,
            cf: 0,
            tf: TtiBlock::encode_text(txt, opt.dh, cct),
            cct, //Needed for Display/Debug without access to GsiBlock
        }
    }

    fn encode_text(txt: &str, dh: bool, cct: CharacterCodeTable) -> Vec<u8> {
        const TF_LENGTH: usize = 112;

        let text = match cct {
            CharacterCodeTable::Latin => iso6937::encode_to_vec(txt),
            CharacterCodeTable::LatinCyrillic => iso8859_5::encode_to_vec(txt),
            CharacterCodeTable::LatinArabic => iso8859_6::encode_to_vec(txt),
            CharacterCodeTable::LatinGreek => iso8859_7::encode_to_vec(txt),
            CharacterCodeTable::LatinHebrew => iso8859_8::encode_to_vec(txt),
        };
        let mut res = Vec::with_capacity(TF_LENGTH);
        if dh {
            res.push(0x0d);
        }
        res.push(0x0b);
        res.push(0x0b);
        res.extend(text);

        // Make sure size does not exceeds 112 bytes, FIXME: and what if!
        let max_size = TF_LENGTH - 3; // 3 trailing teletext codes to add.
        if res.len() > max_size {
            println!("!!! subtitle length is too long, truncating!");
        }
        res.truncate(max_size);
        res.push(0x0A);
        res.push(0x0A);
        res.push(0x8A);
        let padding = TF_LENGTH - res.len();
        res.extend(vec![0x8Fu8; padding]);
        res
    }

    pub fn get_text(&self) -> String {
        let mut result = String::from("");
        let mut first = 0;
        for i in 0..self.tf.len() {
            let c = self.tf[i];
            if match c {
                0x0..=0x1f => true, //TODO: decode teletext control codes
                0x20..=0x7f => false,
                0x80..=0x9f => true, // TODO: decode codes
                0xa0..=0xff => false,
            } {
                if first != i {
                    let data = &self.tf[first..i];
                    let decoded_string = match self.cct {
                        CharacterCodeTable::Latin => iso6937::decode_to_string(data),
                        CharacterCodeTable::LatinCyrillic => iso8859_5::decode_to_string(data),
                        CharacterCodeTable::LatinArabic => iso8859_6::decode_to_string(data),
                        CharacterCodeTable::LatinGreek => iso8859_7::decode_to_string(data),
                        CharacterCodeTable::LatinHebrew => iso8859_8::decode_to_string(data),
                    };
                    result.push_str(&decoded_string);
                }
                if c == 0x8f {
                    break;
                } else if c == 0x8a {
                    result.push_str("\r\n");
                }
                first = i + 1;
            }
        }
        result
    }

    #[allow(clippy::vec_init_then_push)]
    fn serialize(&self) -> Vec<u8> {
        let mut res = vec![];
        res.push(self.sgn);
        res.push((self.sn & 0xff) as u8);
        res.push((self.sn >> 8) as u8);
        res.push(self.ebn);
        res.push(self.cs.serialize());
        res.extend(self.tci.serialize().iter().cloned());
        res.extend(self.tco.serialize().iter().cloned());
        res.push(self.vp);
        res.push(self.jc);
        res.push(self.cf);
        res.extend(self.tf.iter().cloned());
        res
    }
}

impl fmt::Debug for TtiBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "\n{}-->{} sgn:{} sn:{} ebn:{} cs:{:?} vp:{} jc:{} cf:{} [{}]",
            self.tci,
            self.tco,
            self.sgn,
            self.sn,
            self.ebn,
            self.cs,
            self.vp,
            self.jc,
            self.cf,
            self.get_text()
        )
    }
}

impl fmt::Display for TtiBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "\n{} {} {} {} {:?} [{}]",
            self.tci,
            self.sgn,
            self.sn,
            self.ebn,
            self.cs,
            self.get_text()
        )
    }
}
