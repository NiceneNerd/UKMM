pub mod string_ext;

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::LazyLock;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json;

pub static LOCALIZATION: LazyLock<RwLock<Localization>> = LazyLock::new(|| Localization::from(LocLang::English).into());

static DE: &'static str = include_str!("../localization/de.json");
static EN: &'static str = include_str!("../localization/en.json");
static ES: &'static str = include_str!("../localization/es.json");
static FR: &'static str = include_str!("../localization/fr.json");
static IT: &'static str = include_str!("../localization/it.json");
static JA: &'static str = include_str!("../localization/ja.json");
static KO: &'static str = include_str!("../localization/ko.json");
static RU: &'static str = include_str!("../localization/ru.json");
static NL: &'static str = include_str!("../localization/nl.json");
static ZH: &'static str = include_str!("../localization/zh.json");

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LocLang {
    English,
    Dutch,
    French,
    German,
    Italian,
    Japanese,
    Korean,
    Russian,
    SimpleChinese,
    Spanish,
}

impl From<LocLang> for &str {
    fn from(value: LocLang) -> Self {
        match value {
            LocLang::English => "English",
            LocLang::Dutch => "Nederlands",
            LocLang::French => "Français",
            LocLang::German => "Deutsch",
            LocLang::Italian => "Italiano",
            LocLang::Japanese => "日本語",
            LocLang::Korean => "한국어 [韓國語]",
            LocLang::Russian => "Русский язык",
            LocLang::SimpleChinese => "中文",
            LocLang::Spanish => "Español",
        }
    }
}

impl std::fmt::Display for LocLang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

impl LocLang {
    pub fn iter() -> std::slice::Iter<'static, Self> {
        [
            Self::English,
            Self::Dutch,
            Self::French,
            Self::German,
            Self::Italian,
            Self::Japanese,
            Self::Korean,
            Self::Russian,
            Self::SimpleChinese,
            Self::Spanish,
        ]
            .iter()
    }

    #[inline(always)]
    pub fn to_str(self) -> &'static str {
        self.into()
    }
}

pub struct Localization {
    pub language: LocLang,
    strings: DashMap<&'static str, Cow<'static, str>>,
    strings_default: DashMap<&'static str, Cow<'static, str>>,
}

impl<'a> From<LocLang> for Localization {
    fn from(value: LocLang) -> Self {
        Self {
            strings: match value {
                LocLang::English => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&EN)
                    .expect("Invalid English localization")
                    .into_iter()
                    .collect(),
                LocLang::Dutch => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&NL)
                    .expect("Invalid Dutch localization")
                    .into_iter()
                    .collect(),
                LocLang::French => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&FR)
                    .expect("Invalid French localization")
                    .into_iter()
                    .collect(),
                LocLang::German => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&DE)
                    .expect("Invalid German localization")
                    .into_iter()
                    .collect(),
                LocLang::Italian => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&IT)
                    .expect("Invalid Italian localization")
                    .into_iter()
                    .collect(),
                LocLang::Japanese => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&JA)
                    .expect("Invalid Japanese localization")
                    .into_iter()
                    .collect(),
                LocLang::Korean => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&KO)
                    .expect("Invalid Korean localization")
                    .into_iter()
                    .collect(),
                LocLang::Russian => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&RU)
                    .expect("Invalid Russian localization")
                    .into_iter()
                    .collect(),
                LocLang::SimpleChinese => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&ZH)
                    .expect("Invalid SimpleChinese localization")
                    .into_iter()
                    .collect(),
                LocLang::Spanish => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&ES)
                    .expect("Invalid Spanish localization")
                    .into_iter()
                    .collect(),
            },
            language: value,
            strings_default: serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&EN)
                .expect("Invalid English localization")
                .into_iter()
                .collect()
        }
    }
}

impl Localization {
    pub fn get(&self, key: &'static str) -> Cow<'static, str> {
        self.strings.get(&key)
            .map(|v| v.clone())
            .unwrap_or_else(|| self.strings_default.get(&key)
                .map(|v| v.clone())
                .unwrap_or(key.into()))
    }

    pub fn update_language(&mut self, lang: &LocLang) {
        self.strings = match lang {
            LocLang::English => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&EN)
                .expect("Invalid English localization")
                .into_iter()
                .collect(),
            LocLang::Dutch => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&NL)
                .expect("Invalid Dutch localization")
                .into_iter()
                .collect(),
            LocLang::French => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&FR)
                .expect("Invalid French localization")
                .into_iter()
                .collect(),
            LocLang::German => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&DE)
                .expect("Invalid German localization")
                .into_iter()
                .collect(),
            LocLang::Italian => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&IT)
                .expect("Invalid Italian localization")
                .into_iter()
                .collect(),
            LocLang::Japanese => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&JA)
                .expect("Invalid Japanese localization")
                .into_iter()
                .collect(),
            LocLang::Korean => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&KO)
                .expect("Invalid Korean localization")
                .into_iter()
                .collect(),
            LocLang::Russian => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&RU)
                .expect("Invalid Russian localization")
                .into_iter()
                .collect(),
            LocLang::SimpleChinese => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&ZH)
                .expect("Invalid SimpleChinese localization")
                .into_iter()
                .collect(),
            LocLang::Spanish => serde_json::from_str::<HashMap<&'static str, Cow<'static, str>>>(&ES)
                .expect("Invalid Spanish localization")
                .into_iter()
                .collect(),
        };
        self.language = *lang;
    }
}
