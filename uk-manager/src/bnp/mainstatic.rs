use anyhow::{Context, Result};
use roead::byml::Hash;

use super::BnpConverter;

#[inline]
fn key_from_coords(x: f32, y: f32, z: f32) -> String {
    format!("{}{}{}", x.ceil(), y.ceil(), z.ceil())
}

fn get_id(item: &Hash) -> Result<String> {
    fn find_name<'h>(item: &'h Hash) -> &'h str {
        item.iter()
            .find_map(|(k, v)| {
                k.to_lowercase()
                    .contains("name")
                    .then(|| v.as_string().ok().map(|v| v.as_str()))
                    .flatten()
            })
            .unwrap_or("")
    }

    let translate = item
        .get("Translate")
        .context("Mainfield static missing entry translation")?
        .as_hash()?;

    Ok(key_from_coords(
        translate
            .get("X")
            .context("Translate missing X")?
            .as_float()?,
        translate
            .get("Y")
            .context("Translate missing Y")?
            .as_float()?,
        translate
            .get("Z")
            .context("Translate missing Z")?
            .as_float()?,
    ) + find_name(item))
}

impl BnpConverter<'_> {
    pub fn handle_mainfield_static(&self) -> Result<()> {
        Ok(())
    }
}
