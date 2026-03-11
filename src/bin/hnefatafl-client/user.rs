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

use hnefatafl_copenhagen::glicko::{CONFIDENCE_INTERVAL_95, Rating};
use log::error;

use crate::enums::LoggedIn;

#[derive(Clone, Debug)]
pub(crate) struct User {
    pub name: String,
    pub wins: String,
    pub losses: String,
    pub draws: String,
    pub rating: Rating,
    pub logged_in: LoggedIn,
}

impl From<&[&str; 6]> for User {
    fn from(user: &[&str; 6]) -> Self {
        let [name, wins, losses, draws, rating, logged_in] = *user;

        let (mut rating, mut deviation) = rating.split_once("±").unwrap_or_else(|| {
            error!("The ratings has this form: {rating}");
            unreachable!();
        });

        rating = rating.trim();
        deviation = deviation.trim();

        let (Ok(rating), Ok(deviation)) = (rating.parse::<f64>(), deviation.parse::<f64>()) else {
            error!("The ratings has this form: ({rating}, {deviation})");
            unreachable!();
        };

        let logged_in = if "logged_in" == logged_in {
            LoggedIn::Yes
        } else {
            LoggedIn::No
        };

        User {
            name: name.to_string(),
            wins: wins.to_string(),
            losses: losses.to_string(),
            draws: draws.to_string(),
            rating: Rating {
                rating,
                rd: deviation / CONFIDENCE_INTERVAL_95,
            },
            logged_in,
        }
    }
}
