use nk_core::*;
use smartstring::alias::String;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Endian {
    #[serde(rename = "Switch")]
    Little,
    #[serde(rename = "Wii U")]
    Big,
}

impl std::fmt::Display for Endian {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Endian::Little => f.write_str("Little (Switch)"),
            Endian::Big => f.write_str("Big (Wii U)"),
        }
    }
}

impl From<roead::Endian> for Endian {
    fn from(endian: roead::Endian) -> Self {
        match endian {
            roead::Endian::Little => Endian::Little,
            roead::Endian::Big => Endian::Big,
        }
    }
}

impl From<Endian> for roead::Endian {
    fn from(endian: Endian) -> Self {
        match endian {
            Endian::Little => roead::Endian::Little,
            Endian::Big => roead::Endian::Big,
        }
    }
}

impl From<&roead::Endian> for Endian {
    fn from(endian: &roead::Endian) -> Self {
        match endian {
            roead::Endian::Little => Endian::Little,
            roead::Endian::Big => Endian::Big,
        }
    }
}

impl From<&Endian> for roead::Endian {
    fn from(endian: &Endian) -> Self {
        match endian {
            Endian::Little => roead::Endian::Little,
            Endian::Big => roead::Endian::Big,
        }
    }
}

impl From<rstb::Endian> for Endian {
    fn from(endian: rstb::Endian) -> Self {
        match endian {
            rstb::Endian::Little => Self::Little,
            rstb::Endian::Big => Self::Big,
        }
    }
}

impl From<Endian> for rstb::Endian {
    fn from(endian: Endian) -> Self {
        match endian {
            Endian::Little => Self::Little,
            Endian::Big => Self::Big,
        }
    }
}

pub const fn platform_content(endian: Endian) -> &'static str {
    match endian {
        Endian::Little => "01007EF00011E000/romfs",
        Endian::Big => "content",
    }
}

pub const fn platform_aoc(endian: Endian) -> &'static str {
    match endian {
        Endian::Little => "01007EF00011F001/romfs",
        Endian::Big => "aoc/0010",
    }
}

pub const fn platform_prefixes(endian: Endian) -> (&'static str, &'static str) {
    match endian {
        Endian::Little => ("01007EF00011E000/romfs", "01007EF00011F001/romfs"),
        Endian::Big => ("content", "aoc/0010"),
    }
}
