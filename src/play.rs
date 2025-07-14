use std::{fmt, str::FromStr};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::{board::BoardSize, role::Role, time::TimeSettings};

pub const BOARD_LETTERS: &str = "ABCDEFGHIJKLM";

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PlayRecord {
    pub play: Option<Plae>,
    pub attacker_time: TimeSettings,
    pub defender_time: TimeSettings,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Plae {
    Play(Play),
    AttackerResigns,
    DefenderResigns,
}

impl fmt::Display for Plae {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Play(play) => writeln!(f, "play {} {} {}", play.role, play.from, play.to),
            Self::AttackerResigns => writeln!(f, "play attacker resigns _"),
            Self::DefenderResigns => writeln!(f, "play defender resigns _"),
        }
    }
}

impl Plae {
    // Todo: can the player resign?
    /// # Errors
    ///
    /// If you try to convert an illegal character or you don't get vertex-vertex.
    pub fn from_str_(play: &str, role: &Role) -> anyhow::Result<Self> {
        let Some((from, to)) = play.split_once('-') else {
            return Err(anyhow::Error::msg("expected: vertex-vertex"));
        };

        Ok(Self::Play(Play {
            role: *role,
            from: Vertex::from_str(from)?,
            to: Vertex::from_str(to)?,
        }))
    }
}

impl TryFrom<Vec<&str>> for Plae {
    type Error = anyhow::Error;

    fn try_from(args: Vec<&str>) -> Result<Self, Self::Error> {
        let error_str = "expected: 'play ROLE FROM TO' or 'play ROLE resign'";

        if args.len() < 3 {
            return Err(anyhow::Error::msg(error_str));
        }

        let role = Role::from_str(args[1])?;
        if args[2] == "resigns" {
            if role == Role::Defender {
                return Ok(Self::DefenderResigns);
            }

            return Ok(Self::AttackerResigns);
        }

        if args.len() < 4 {
            return Err(anyhow::Error::msg(error_str));
        }

        Ok(Self::Play(Play {
            role: Role::from_str(args[1])?,
            from: Vertex::from_str(args[2])?,
            to: Vertex::from_str(args[3])?,
        }))
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Plays(pub Vec<PlayRecord>);

impl fmt::Display for Plays {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.0.is_empty() {
            writeln!(f)?;
        }

        for play in &self.0 {
            if let Some(play) = &play.play {
                write!(f, "    {play}")?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Play {
    pub role: Role,
    pub from: Vertex,
    pub to: Vertex,
}

impl fmt::Display for Play {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} from {} to {}", self.role, self.from, self.to)
    }
}

#[derive(Default)]
pub struct Captures(pub Vec<Vertex>);

impl fmt::Display for Captures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for vertex in &self.0 {
            write!(f, "{vertex} ")?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Vertex {
    #[serde(default)]
    pub board_size: BoardSize,
    pub x: usize,
    pub y: usize,
}

impl fmt::Display for Vertex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let letters = if self.board_size == BoardSize::Size11 {
            &BOARD_LETTERS.to_lowercase()
        } else {
            BOARD_LETTERS
        };

        let board_size: usize = self.board_size.into();

        write!(
            f,
            "{}{}",
            letters.chars().collect::<Vec<_>>()[self.x],
            board_size - self.y
        )
    }
}

impl FromStr for Vertex {
    type Err = anyhow::Error;

    fn from_str(vertex: &str) -> anyhow::Result<Self> {
        let mut chars = vertex.chars();

        if let Some(mut ch) = chars.next() {
            let board_size = if ch.is_lowercase() { 11 } else { 13 };

            ch = ch.to_ascii_uppercase();
            let x = BOARD_LETTERS[..board_size]
                .find(ch)
                .context("play: the first letter is not a legal char")?;

            let mut y = chars.as_str().parse()?;
            if y > 0 && y <= board_size {
                y = board_size - y;
                return Ok(Self {
                    board_size: board_size.try_into()?,
                    x,
                    y,
                });
            }
        }

        Err(anyhow::Error::msg("play: invalid coordinate"))
    }
}

impl Vertex {
    #[must_use]
    pub fn fmt_other(&self) -> String {
        let board_size: usize = self.board_size.into();

        format!(
            "{}{}",
            BOARD_LETTERS.chars().collect::<Vec<_>>()[self.x],
            board_size - self.y
        )
    }

    #[must_use]
    pub fn up(&self) -> Option<Vertex> {
        if self.y > 0 {
            Some(Vertex {
                board_size: self.board_size,
                x: self.x,
                y: self.y - 1,
            })
        } else {
            None
        }
    }

    #[must_use]
    pub fn left(&self) -> Option<Vertex> {
        if self.x > 0 {
            Some(Vertex {
                board_size: self.board_size,
                x: self.x - 1,
                y: self.y,
            })
        } else {
            None
        }
    }

    #[must_use]
    pub fn down(&self) -> Option<Vertex> {
        if self.y < 10 {
            Some(Vertex {
                board_size: self.board_size,
                x: self.x,
                y: self.y + 1,
            })
        } else {
            None
        }
    }

    #[must_use]
    pub fn right(&self) -> Option<Vertex> {
        if self.x < 10 {
            Some(Vertex {
                board_size: self.board_size,
                x: self.x + 1,
                y: self.y,
            })
        } else {
            None
        }
    }

    #[must_use]
    pub fn touches_wall(&self) -> bool {
        self.x == 0 || self.x == 10 || self.y == 0 || self.y == 10
    }
}
