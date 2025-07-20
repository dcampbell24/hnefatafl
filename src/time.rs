use std::fmt;

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Time {
    pub add_seconds: i64,
    pub milliseconds_left: i64,
}

impl Time {
    #[must_use]
    pub fn fmt_shorthand(&self) -> String {
        let minutes = self.milliseconds_left / 60_000;
        let mut seconds = self.milliseconds_left % 60_000;
        seconds /= 1_000;

        format!("{minutes:02}:{seconds:02}")
    }
}

impl Default for Time {
    fn default() -> Self {
        Self {
            add_seconds: 10,
            milliseconds_left: 15 * 60_000,
        }
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let day = 24 * 60 * 60 * 1_000;
        let hour = 60 * 60 * 1_000;
        let minute = 60 * 1_000;

        let days = self.milliseconds_left / day;
        let hours = (self.milliseconds_left % day) / hour;
        let minutes = (self.milliseconds_left % hour) / minute;

        let add_hours = self.add_seconds / (60 * 60);
        let add_minutes = (self.add_seconds % (60 * 60)) / 60;
        let add_seconds = self.add_seconds % 60;

        if days == 0 {
            write!(
                f,
                "{hours:02}:{minutes:02} | {add_hours:02}:{add_minutes:02}:{add_seconds:02}"
            )
        } else {
            write!(
                f,
                "{days} {hours:02}:{minutes:02} | {add_hours:02}:{add_minutes:02}:{add_seconds:02}"
            )
        }
    }
}

#[derive(Clone, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum TimeSettings {
    Timed(Time),
    UnTimed,
}

impl TimeSettings {
    #[must_use]
    pub fn fmt_shorthand(&self) -> String {
        match self {
            Self::Timed(time) => time.fmt_shorthand(),
            Self::UnTimed => "-".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct TimeLeft {
    pub milliseconds_left: i64,
}

impl fmt::Display for TimeLeft {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let minutes = self.milliseconds_left / 60_000;
        let mut seconds = self.milliseconds_left % 60_000;
        seconds /= 1_000;

        write!(f, "{minutes:02}:{seconds:02}")
    }
}

impl From<Time> for TimeLeft {
    fn from(time: Time) -> Self {
        TimeLeft {
            milliseconds_left: time.milliseconds_left,
        }
    }
}

impl TryFrom<TimeSettings> for TimeLeft {
    type Error = anyhow::Error;

    fn try_from(time: TimeSettings) -> Result<TimeLeft, anyhow::Error> {
        match time {
            TimeSettings::Timed(time) => Ok(TimeLeft {
                milliseconds_left: time.milliseconds_left,
            }),
            TimeSettings::UnTimed => Err(anyhow::Error::msg("the time settings are un-timed")),
        }
    }
}

impl Default for TimeSettings {
    fn default() -> Self {
        Self::Timed(Time {
            ..Default::default()
        })
    }
}

impl fmt::Debug for TimeSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timed(time) => {
                write!(f, "fischer {} {}", time.milliseconds_left, time.add_seconds)
            }
            Self::UnTimed => write!(f, "un-timed _ _"),
        }
    }
}

impl fmt::Display for TimeSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timed(time) => write!(f, "{time}"),
            Self::UnTimed => write!(f, "-"),
        }
    }
}

impl From<TimeSettings> for bool {
    fn from(time_settings: TimeSettings) -> Self {
        match time_settings {
            TimeSettings::Timed(_) => true,
            TimeSettings::UnTimed => false,
        }
    }
}

impl TryFrom<Vec<&str>> for TimeSettings {
    type Error = anyhow::Error;

    fn try_from(args: Vec<&str>) -> anyhow::Result<Self> {
        let err_msg = "expected: 'time_settings un-timed' or 'time_settings fischer MILLISECONDS ADD_SECONDS'";

        if Some("un-timed").as_ref() == args.get(1) {
            return Ok(Self::UnTimed);
        }

        if args.len() < 4 {
            return Err(anyhow::Error::msg(err_msg));
        }

        if "fischer" == args[1] {
            let arg_2 = args[2]
                .parse::<i64>()
                .context("time_settings: arg 2 is not an integer")?;

            let arg_3 = args[3]
                .parse::<i64>()
                .context("time_settings: arg 3 is not an integer")?;

            Ok(Self::Timed(Time {
                add_seconds: arg_3,
                milliseconds_left: arg_2,
            }))
        } else {
            Err(anyhow::Error::msg(err_msg))
        }
    }
}
