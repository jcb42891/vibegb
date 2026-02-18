mod emu;

pub use emu::*;

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

const MIN_ROM_SIZE: usize = 0x150;
const TITLE_START: usize = 0x134;
const CGB_FLAG_ADDR: usize = 0x143;
const NEW_LICENSEE_START: usize = 0x144;
const SGB_FLAG_ADDR: usize = 0x146;
const CARTRIDGE_TYPE_ADDR: usize = 0x147;
const ROM_SIZE_ADDR: usize = 0x148;
const RAM_SIZE_ADDR: usize = 0x149;
const DESTINATION_CODE_ADDR: usize = 0x14A;
const OLD_LICENSEE_ADDR: usize = 0x14B;
const MASK_ROM_VERSION_ADDR: usize = 0x14C;
const HEADER_CHECKSUM_ADDR: usize = 0x14D;
const GLOBAL_CHECKSUM_START: usize = 0x14E;
const LOGO_START: usize = 0x104;
const LOGO_END_EXCLUSIVE: usize = 0x134;

const NINTENDO_LOGO: [u8; 48] = [
    0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D,
    0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99,
    0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rom {
    pub data: Vec<u8>,
    pub header: RomHeader,
    pub path: Option<PathBuf>,
}

impl Rom {
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, RomError> {
        let header = RomHeader::parse(&data).map_err(RomError::Header)?;
        Ok(Self {
            data,
            header,
            path: None,
        })
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, RomError> {
        let path = path.as_ref();
        let data = fs::read(path).map_err(|source| RomError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let mut rom = Self::from_bytes(data)?;
        rom.path = Some(path.to_path_buf());
        Ok(rom)
    }
}

#[derive(Debug)]
pub enum RomError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Header(HeaderError),
}

impl Display for RomError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "failed to read ROM '{}': {}", path.display(), source)
            }
            Self::Header(err) => write!(f, "{err}"),
        }
    }
}

impl Error for RomError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Header(err) => Some(err),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomHeader {
    pub title: String,
    pub cgb_mode: CgbMode,
    pub sgb_supported: bool,
    pub cartridge_type: u8,
    pub rom_size_code: u8,
    pub rom_size_bytes: Option<usize>,
    pub ram_size_code: u8,
    pub ram_size_bytes: Option<usize>,
    pub destination_code: u8,
    pub old_licensee_code: u8,
    pub new_licensee_code: Option<String>,
    pub mask_rom_version: u8,
    pub header_checksum: u8,
    pub calculated_header_checksum: u8,
    pub global_checksum: u16,
}

impl RomHeader {
    pub fn parse(data: &[u8]) -> Result<Self, HeaderError> {
        if data.len() < MIN_ROM_SIZE {
            return Err(HeaderError::RomTooSmall {
                actual: data.len(),
                minimum: MIN_ROM_SIZE,
            });
        }

        if data[LOGO_START..LOGO_END_EXCLUSIVE] != NINTENDO_LOGO {
            return Err(HeaderError::InvalidNintendoLogo);
        }

        let calculated_header_checksum = calculate_header_checksum(data);
        let header_checksum = data[HEADER_CHECKSUM_ADDR];
        if calculated_header_checksum != header_checksum {
            return Err(HeaderError::InvalidHeaderChecksum {
                expected: calculated_header_checksum,
                actual: header_checksum,
            });
        }

        let cgb_flag = data[CGB_FLAG_ADDR];
        let old_licensee_code = data[OLD_LICENSEE_ADDR];

        let new_licensee_code = if old_licensee_code == 0x33 {
            let raw = &data[NEW_LICENSEE_START..=NEW_LICENSEE_START + 1];
            Some(
                raw.iter()
                    .map(|byte| sanitize_printable(*byte))
                    .collect::<String>(),
            )
        } else {
            None
        };

        Ok(Self {
            title: parse_title(data, cgb_flag),
            cgb_mode: CgbMode::from_flag(cgb_flag),
            sgb_supported: data[SGB_FLAG_ADDR] == 0x03,
            cartridge_type: data[CARTRIDGE_TYPE_ADDR],
            rom_size_code: data[ROM_SIZE_ADDR],
            rom_size_bytes: rom_size_bytes(data[ROM_SIZE_ADDR]),
            ram_size_code: data[RAM_SIZE_ADDR],
            ram_size_bytes: ram_size_bytes(data[RAM_SIZE_ADDR]),
            destination_code: data[DESTINATION_CODE_ADDR],
            old_licensee_code,
            new_licensee_code,
            mask_rom_version: data[MASK_ROM_VERSION_ADDR],
            header_checksum,
            calculated_header_checksum,
            global_checksum: u16::from_be_bytes([
                data[GLOBAL_CHECKSUM_START],
                data[GLOBAL_CHECKSUM_START + 1],
            ]),
        })
    }

    pub fn cartridge_type_name(&self) -> &'static str {
        cartridge_type_name(self.cartridge_type)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgbMode {
    DmgOnly,
    CgbEnhanced,
    CgbOnly,
    Unknown(u8),
}

impl CgbMode {
    fn from_flag(flag: u8) -> Self {
        match flag {
            0x80 => Self::CgbEnhanced,
            0xC0 => Self::CgbOnly,
            0x00 => Self::DmgOnly,
            other => Self::Unknown(other),
        }
    }
}

impl Display for CgbMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DmgOnly => write!(f, "DMG only"),
            Self::CgbEnhanced => write!(f, "CGB enhanced"),
            Self::CgbOnly => write!(f, "CGB only"),
            Self::Unknown(value) => write!(f, "unknown (0x{value:02X})"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderError {
    RomTooSmall { actual: usize, minimum: usize },
    InvalidNintendoLogo,
    InvalidHeaderChecksum { expected: u8, actual: u8 },
}

impl Display for HeaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RomTooSmall { actual, minimum } => {
                write!(
                    f,
                    "ROM too small: got {actual} bytes, need at least {minimum}"
                )
            }
            Self::InvalidNintendoLogo => write!(f, "invalid Nintendo logo in ROM header"),
            Self::InvalidHeaderChecksum { expected, actual } => write!(
                f,
                "invalid header checksum: expected 0x{expected:02X}, got 0x{actual:02X}"
            ),
        }
    }
}

impl Error for HeaderError {}

fn parse_title(data: &[u8], cgb_flag: u8) -> String {
    let title_end = if matches!(cgb_flag, 0x80 | 0xC0) {
        0x142
    } else {
        CGB_FLAG_ADDR
    };

    let raw = &data[TITLE_START..=title_end];
    let length = raw
        .iter()
        .position(|byte| *byte == 0x00)
        .unwrap_or(raw.len());
    raw[..length]
        .iter()
        .map(|byte| sanitize_printable(*byte))
        .collect::<String>()
        .trim_end()
        .to_string()
}

fn sanitize_printable(byte: u8) -> char {
    if (0x20..=0x7E).contains(&byte) {
        char::from(byte)
    } else {
        '?'
    }
}

fn calculate_header_checksum(data: &[u8]) -> u8 {
    let mut checksum = 0u8;
    for byte in &data[TITLE_START..=MASK_ROM_VERSION_ADDR] {
        checksum = checksum.wrapping_sub(*byte).wrapping_sub(1);
    }
    checksum
}

fn rom_size_bytes(code: u8) -> Option<usize> {
    match code {
        0x00 => Some(32 * 1024),
        0x01 => Some(64 * 1024),
        0x02 => Some(128 * 1024),
        0x03 => Some(256 * 1024),
        0x04 => Some(512 * 1024),
        0x05 => Some(1024 * 1024),
        0x06 => Some(2 * 1024 * 1024),
        0x07 => Some(4 * 1024 * 1024),
        0x08 => Some(8 * 1024 * 1024),
        0x52 => Some(1152 * 1024),
        0x53 => Some(1280 * 1024),
        0x54 => Some(1536 * 1024),
        _ => None,
    }
}

fn ram_size_bytes(code: u8) -> Option<usize> {
    match code {
        0x00 => Some(0),
        0x01 => Some(2 * 1024),
        0x02 => Some(8 * 1024),
        0x03 => Some(32 * 1024),
        0x04 => Some(128 * 1024),
        0x05 => Some(64 * 1024),
        _ => None,
    }
}

fn cartridge_type_name(code: u8) -> &'static str {
    match code {
        0x00 => "ROM ONLY",
        0x01 => "MBC1",
        0x02 => "MBC1+RAM",
        0x03 => "MBC1+RAM+BATTERY",
        0x05 => "MBC2",
        0x06 => "MBC2+BATTERY",
        0x08 => "ROM+RAM",
        0x09 => "ROM+RAM+BATTERY",
        0x0B => "MMM01",
        0x0C => "MMM01+RAM",
        0x0D => "MMM01+RAM+BATTERY",
        0x0F => "MBC3+TIMER+BATTERY",
        0x10 => "MBC3+TIMER+RAM+BATTERY",
        0x11 => "MBC3",
        0x12 => "MBC3+RAM",
        0x13 => "MBC3+RAM+BATTERY",
        0x19 => "MBC5",
        0x1A => "MBC5+RAM",
        0x1B => "MBC5+RAM+BATTERY",
        0x1C => "MBC5+RUMBLE",
        0x1D => "MBC5+RUMBLE+RAM",
        0x1E => "MBC5+RUMBLE+RAM+BATTERY",
        0x20 => "MBC6",
        0x22 => "MBC7+SENSOR+RUMBLE+RAM+BATTERY",
        0xFC => "POCKET CAMERA",
        0xFD => "BANDAI TAMA5",
        0xFE => "HuC3",
        0xFF => "HuC1+RAM+BATTERY",
        _ => "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_header_metadata() {
        let rom = make_test_rom();
        let header = RomHeader::parse(&rom).expect("valid test ROM should parse");

        assert_eq!(header.title, "VIBEGB TEST");
        assert_eq!(header.cgb_mode, CgbMode::CgbEnhanced);
        assert!(header.sgb_supported);
        assert_eq!(header.cartridge_type, 0x01);
        assert_eq!(header.cartridge_type_name(), "MBC1");
        assert_eq!(header.rom_size_bytes, Some(32 * 1024));
        assert_eq!(header.ram_size_bytes, Some(8 * 1024));
        assert_eq!(header.old_licensee_code, 0x33);
        assert_eq!(header.new_licensee_code.as_deref(), Some("01"));
        assert_eq!(header.global_checksum, 0x1234);
        assert_eq!(header.header_checksum, header.calculated_header_checksum);
    }

    #[test]
    fn rejects_roms_smaller_than_header() {
        let err = RomHeader::parse(&vec![0; MIN_ROM_SIZE - 1]).expect_err("expected error");
        assert!(matches!(
            err,
            HeaderError::RomTooSmall {
                actual,
                minimum: MIN_ROM_SIZE
            } if actual == MIN_ROM_SIZE - 1
        ));
    }

    #[test]
    fn rejects_invalid_logo() {
        let mut rom = make_test_rom();
        rom[LOGO_START] ^= 0xFF;
        let err = RomHeader::parse(&rom).expect_err("expected error");
        assert!(matches!(err, HeaderError::InvalidNintendoLogo));
    }

    #[test]
    fn rejects_invalid_checksum() {
        let mut rom = make_test_rom();
        rom[HEADER_CHECKSUM_ADDR] ^= 0x01;
        let err = RomHeader::parse(&rom).expect_err("expected error");
        assert!(matches!(err, HeaderError::InvalidHeaderChecksum { .. }));
    }

    fn make_test_rom() -> Vec<u8> {
        let mut rom = vec![0; 0x8000];
        rom[LOGO_START..LOGO_END_EXCLUSIVE].copy_from_slice(&NINTENDO_LOGO);

        let title = b"VIBEGB TEST";
        rom[TITLE_START..TITLE_START + title.len()].copy_from_slice(title);
        rom[CGB_FLAG_ADDR] = 0x80;
        rom[NEW_LICENSEE_START] = b'0';
        rom[NEW_LICENSEE_START + 1] = b'1';
        rom[SGB_FLAG_ADDR] = 0x03;
        rom[CARTRIDGE_TYPE_ADDR] = 0x01;
        rom[ROM_SIZE_ADDR] = 0x00;
        rom[RAM_SIZE_ADDR] = 0x02;
        rom[DESTINATION_CODE_ADDR] = 0x01;
        rom[OLD_LICENSEE_ADDR] = 0x33;
        rom[MASK_ROM_VERSION_ADDR] = 0x00;
        rom[HEADER_CHECKSUM_ADDR] = calculate_header_checksum(&rom);
        rom[GLOBAL_CHECKSUM_START] = 0x12;
        rom[GLOBAL_CHECKSUM_START + 1] = 0x34;
        rom
    }
}
