use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Locale {
    #[default]
    English,
    Chinese,
    Spanish,
    Arabic,
    Indonesian,
    PortugueseBr,
    PortuguesePt,
    French,
    Japanese,
    Russian,
    German,
    Icelandic,
    IcelandicRunic,
    Swedish,
}

impl Locale {
    #[must_use]
    pub fn txt(self) -> String {
        match self {
            Self::English => "en-US".to_string(),
            Self::Chinese => "zh-CN".to_string(),
            Self::Spanish => "es".to_string(),
            Self::Arabic => "ar".to_string(),
            Self::Indonesian => "id".to_string(),
            Self::PortugueseBr => "pt-BR".to_string(),
            Self::PortuguesePt => "pt-PT".to_string(),
            Self::French => "fr".to_string(),
            Self::Japanese => "ja".to_string(),
            Self::Russian => "ru".to_string(),
            Self::German => "de".to_string(),
            Self::Icelandic => "is-IS".to_string(),
            Self::IcelandicRunic => "is-RU".to_string(),
            Self::Swedish => "sv-SE".to_string(),
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::English => write!(f, "English (United States)"),
            Self::Chinese => write!(f, "中文 (中国)"),
            Self::Spanish => write!(f, "Español"),
            Self::Arabic => write!(f, "العربية"),
            Self::Indonesian => write!(f, "bahasa Indonesia"),
            Self::PortugueseBr => write!(f, "Português (Brazil)"),
            Self::PortuguesePt => write!(f, "Português (Portugal)"),
            Self::French => write!(f, "Français"),
            Self::Japanese => write!(f, "日本人"),
            Self::Russian => write!(f, "Русский"),
            Self::German => write!(f, "Deutsch"),
            Self::Icelandic => write!(f, "Íslenska"),
            Self::IcelandicRunic => write!(f, "ᛇᛋᛚᛂᚿᛋᚴᛁ ᚱᚤᛐᚢᚱᛁᚿᚿ (Íslenska Rúturinn)"),
            Self::Swedish => write!(f, "Svenska"),
        }
    }
}
