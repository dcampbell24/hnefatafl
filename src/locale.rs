// This file is part of hnefatafl-copenhagen.
//
// hnefatafl-copenhagen is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// hnefatafl-copenhagen is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
    Korean,
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
            Self::Korean => "ko".to_string(),
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
            Self::PortugueseBr => write!(f, "Português (Brasil)"),
            Self::PortuguesePt => write!(f, "Português (Portugal)"),
            Self::French => write!(f, "Français"),
            Self::Japanese => write!(f, "日本人"),
            Self::Russian => write!(f, "Русский"),
            Self::German => write!(f, "Deutsch"),
            Self::Icelandic => write!(f, "Íslenska"),
            Self::IcelandicRunic => write!(f, "ᛇᛋᛚᛂᚿᛋᚴᛁ ᚱᚤᛐᚢᚱᛁᚿᚿ (Íslenska Rúturinn)"),
            Self::Swedish => write!(f, "Svenska"),
            Self::Korean => write!(f, "한국인"),
        }
    }
}
