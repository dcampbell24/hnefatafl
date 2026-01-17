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

use std::{net::IpAddr, sync::mpsc::Sender};

#[derive(Debug)]
pub(crate) struct RemoveConnection {
    pub address: IpAddr,
    pub tx: Sender<(String, Option<Sender<String>>)>,
}

impl Drop for RemoveConnection {
    fn drop(&mut self) {
        let _ok = self
            .tx
            .send((format!("0 server connection_remove {}", self.address), None));
    }
}
