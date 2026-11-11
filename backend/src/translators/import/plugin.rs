//! The translation catalog built into `ArknightsLocalization.RichTextFix.dll`.
//!
//! The DLL embeds a gzip-compressed JSON object `{ role: { english: russian } }`
//! (`story`, `skill`, `operator`, ..., `ambiguous:<role>`, `id:*`). For a text
//! component with a role, the plugin looks there BEFORE the txt dictionaries,
//! so an edit through the dictionaries may not show for such a text
//! (translators/README.md, "XUnity dictionary and plugin format"). The resource is found by scanning for gzip
//! streams and keeping the one that decodes to that object; the DLL is only
//! read, never loaded.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

/// `role -> (plain English -> Russian)`; an `ambiguous:<role>` entry's values are empty.
#[derive(Debug, Default)]
pub struct PluginCatalog {
    pub roles: HashMap<String, HashMap<String, String>>,
}

impl PluginCatalog {
    pub fn load(dll: &Path) -> anyhow::Result<Self> {
        let bytes = std::fs::read(dll)?;
        Self::from_bytes(&bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        const MAGIC: [u8; 3] = [0x1f, 0x8b, 0x08];
        let mut at = 0;
        while let Some(found) = bytes[at..].windows(3).position(|w| w == MAGIC) {
            let start = at + found;
            at = start + 1;
            let mut json = Vec::new();
            if flate2::read::GzDecoder::new(&bytes[start..])
                .read_to_end(&mut json)
                .is_err()
            {
                continue;
            }
            let Ok(value) = serde_json::from_slice::<serde_json::Value>(&json) else {
                continue;
            };
            let Some(object) = value.as_object() else {
                continue;
            };
            if !object.contains_key("story") {
                continue;
            }
            let mut roles = HashMap::new();
            for (role, entries) in object {
                let Some(entries) = entries.as_object() else {
                    continue;
                };
                let map = entries
                    .iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_owned()))
                    .collect();
                roles.insert(role.clone(), map);
            }
            return Ok(Self { roles });
        }
        anyhow::bail!("no translation catalog found in the plugin DLL")
    }

    pub fn role(&self, role: &str) -> Option<&HashMap<String, String>> {
        self.roles.get(role)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn finds_the_catalog_among_other_bytes() {
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(r#"{"story":{"Hi.":"Привет."},"ambiguous:story":{"Why?":""}}"#.as_bytes())
            .unwrap();
        let mut dll = b"MZ\x1f\x8b\x08 not gzip".to_vec();
        dll.extend(gz.finish().unwrap());
        dll.extend(b"tail");
        let c = PluginCatalog::from_bytes(&dll).unwrap();
        assert_eq!(c.role("story").unwrap()["Hi."], "Привет.");
        assert!(c.role("ambiguous:story").unwrap().contains_key("Why?"));
    }
}
