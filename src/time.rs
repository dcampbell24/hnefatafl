use std::fmt;

use anyhow::Context;
use serde::{Deserialize, Serialize};

const DAY: i64 = 24 * 60 * 60 * 1_000;
const HOUR: i64 = 60 * 60 * 1_000;
const MINUTE: i64 = 60 * 1_000;
const SECOND: i64 = 1_000;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Time {
    pub add_seconds: i64,
    pub milliseconds_left: i64,
}

impl Time {
    #[must_use]
    pub fn time_left(&self) -> String {
        time_left(self.milliseconds_left)
    }
}

impl Default for Time {
    fn default() -> Self {
        Self {
            add_seconds: 10,
            milliseconds_left: 15 * MINUTE,
        }
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let days = self.milliseconds_left / DAY;
        let hours = (self.milliseconds_left % DAY) / HOUR;
        let minutes = (self.milliseconds_left % HOUR) / MINUTE;
        let seconds = (self.milliseconds_left % MINUTE) / SECOND;

        let add_hours = self.add_seconds / (60 * 60);
        let add_minutes = (self.add_seconds % (60 * 60)) / 60;
        let add_seconds = self.add_seconds % 60;

        if days == 0 {
            write!(
                f,
                "{hours:02}:{minutes:02}:{seconds:02} | {add_hours:02}:{add_minutes:02}:{add_seconds:02}"
            )
        } else {
            write!(
                f,
                "{days} {hours:02}:{minutes:02}:{seconds:02} | {add_hours:02}:{add_minutes:02}:{add_seconds:02}"
            )
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TimeEnum {
    Classical,
    Long,
    #[default]
    Rapid,
    VeryLong,
}

impl fmt::Display for TimeEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Classical => write!(f, "00:30:00 | 00:00:20"),
            Self::Long => write!(f, "3 00:00:00 | 6:00:00"),
            Self::Rapid => write!(f, "00:15:00 | 00:00:10 "),
            Self::VeryLong => write!(f, "7 12:00:00 | 15:00:00 "),
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
    pub fn time_left(&self) -> String {
        match self {
            Self::Timed(time) => time.time_left(),
            Self::UnTimed => "-".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct TimeLeft {
    pub milliseconds_left: i64,
}

impl fmt::Display for TimeLeft {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", time_left(self.milliseconds_left))
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

fn time_left(milliseconds_left: i64) -> String {
    let days = milliseconds_left / DAY;
    let hours = (milliseconds_left % DAY) / HOUR;
    let minutes = (milliseconds_left % HOUR) / MINUTE;
    let seconds = (milliseconds_left % MINUTE) / SECOND;

    if days == 0 {
        if hours == 0 {
            if minutes == 0 {
                format!("{seconds:02}")
            } else {
                format!("{minutes:02}:{seconds:02}")
            }
        } else {
            format!("{hours:02}:{minutes:02}:{seconds:02}")
        }
    } else {
        format!("{days} {hours:02}:{minutes:02}:{seconds:02}")
    }
}
