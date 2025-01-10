use std::borrow::Cow;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json;
use uk_content::constants::Language;

static DE: &'static str = include_str!("../../../localization/de.json");
static EN: &'static str = include_str!("../../../localization/en.json");
static ES: &'static str = include_str!("../../../localization/es.json");
static FR: &'static str = include_str!("../../../localization/fr.json");
static IT: &'static str = include_str!("../../../localization/it.json");
static JA: &'static str = include_str!("../../../localization/ja.json");
static KO: &'static str = include_str!("../../../localization/ko.json");
static RU: &'static str = include_str!("../../../localization/ru.json");
static NL: &'static str = include_str!("../../../localization/nl.json");
static ZH: &'static str = include_str!("../../../localization/zh.json");

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

impl From<Language> for LocLang {
    fn from(value: Language) -> Self {
        match value {
            Language::CNzh | Language::TWzh => Self::SimpleChinese,
            Language::EUde => Self::German,
            Language::EUen | Language::USen => Self::English,
            Language::EUes | Language::USes => Self::Spanish,
            Language::EUfr | Language::USfr => Self::French,
            Language::EUit => Self::Italian,
            Language::EUnl => Self::Dutch,
            Language::EUru => Self::Russian,
            Language::JPja => Self::Japanese,
            Language::KRko => Self::Korean,
        }
    }
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

pub struct Localization<'a> {
    pub language: LocLang,
    strings: DashMap<&'a str, Cow<'a, str>>
}

impl<'a> From<LocLang> for Localization<'a> {
    fn from(value: LocLang) -> Self {
        Self {
            strings: match value {
                LocLang::English => serde_json::from_str(&EN).expect("Invalid English localization"),
                LocLang::Dutch => serde_json::from_str(&NL).expect("Invalid Dutch localization"),
                LocLang::French => serde_json::from_str(&FR).expect("Invalid French localization"),
                LocLang::German => serde_json::from_str(&DE).expect("Invalid German localization"),
                LocLang::Italian => serde_json::from_str(&IT).expect("Invalid Italian localization"),
                LocLang::Japanese => serde_json::from_str(&JA).expect("Invalid Japanese localization"),
                LocLang::Korean => serde_json::from_str(&KO).expect("Invalid Korean localization"),
                LocLang::Russian => serde_json::from_str(&RU).expect("Invalid Russian localization"),
                LocLang::SimpleChinese => serde_json::from_str(&ZH).expect("Invalid SimpleChinese localization"),
                LocLang::Spanish => serde_json::from_str(&ES).expect("Invalid Spanish localization")
            },
            language: value
        }
    }
}

impl<'a> Localization<'a> {
    pub fn get(&self, key: &'a str) -> Cow<'a, str> {
        self.strings.get(&key)
            .map(|v| v.clone())
            .unwrap_or(key.into())
    }

    pub fn update_language(&mut self, lang: &LocLang) {
        self.strings = match lang {
            LocLang::English => serde_json::from_str(&EN).expect("Invalid English localization"),
            LocLang::Dutch => serde_json::from_str(&NL).expect("Invalid Dutch localization"),
            LocLang::French => serde_json::from_str(&FR).expect("Invalid French localization"),
            LocLang::German => serde_json::from_str(&DE).expect("Invalid German localization"),
            LocLang::Italian => serde_json::from_str(&IT).expect("Invalid Italian localization"),
            LocLang::Japanese => serde_json::from_str(&JA).expect("Invalid Japanese localization"),
            LocLang::Korean => serde_json::from_str(&KO).expect("Invalid Korean localization"),
            LocLang::Russian => serde_json::from_str(&RU).expect("Invalid Russian localization"),
            LocLang::SimpleChinese => serde_json::from_str(&ZH).expect("Invalid SimpleChinese localization"),
            LocLang::Spanish => serde_json::from_str(&ES).expect("Invalid Spanish localization")
        };
        self.language = *lang;
    }
}
