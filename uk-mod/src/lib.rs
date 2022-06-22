#![feature(let_chains)]
pub mod data;
pub(crate) mod resource;

pub const fn platform_content(endian: uk_content::prelude::Endian) -> &'static str {
    match endian {
        uk_content::prelude::Endian::Little => "01007EF00011E000/romfs",
        uk_content::prelude::Endian::Big => "content",
    }
}

pub const fn platform_aoc(endian: uk_content::prelude::Endian) -> &'static str {
    match endian {
        uk_content::prelude::Endian::Little => "01007EF00011F001/romfs",
        uk_content::prelude::Endian::Big => "aoc/0010",
    }
}

pub const fn platform_prefixes(
    endian: uk_content::prelude::Endian,
) -> (&'static str, &'static str) {
    match endian {
        uk_content::prelude::Endian::Little => ("01007EF00011E000/romfs", "01007EF00011F001/romfs"),
        uk_content::prelude::Endian::Big => ("content", "aoc/0010"),
    }
}
