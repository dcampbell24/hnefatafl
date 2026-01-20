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

use std::{ops::Not, sync::mpsc};

use hnefatafl_copenhagen::{
    Id,
    ai::GenerateMove,
    board::BoardSize,
    draw::Draw,
    locale::Locale,
    play::Vertex,
    role::Role,
    server_game::ArchivedGame,
    time::TimeEnum,
    tree::{Node, Tree},
};
use iced::widget::text_editor;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub(crate) enum Coordinates {
    Hide,
    #[default]
    Show,
}

impl Not for Coordinates {
    type Output = Coordinates;

    fn not(self) -> Self::Output {
        match self {
            Self::Hide => Self::Show,
            Self::Show => Self::Hide,
        }
    }
}

impl From<bool> for Coordinates {
    fn from(value: bool) -> Self {
        if value { Self::Show } else { Self::Hide }
    }
}

impl From<Coordinates> for bool {
    fn from(coordinates: Coordinates) -> Self {
        match coordinates {
            Coordinates::Show => true,
            Coordinates::Hide => false,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum JoinGame {
    Cancel,
    Join,
    None,
    Resume,
    Watch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LoggedIn {
    No,
    None,
    Yes,
}

#[derive(Clone, Debug)]
pub(crate) enum Message {
    AccountSettings,
    ArchivedGames(Vec<ArchivedGame>),
    ArchivedGamesGet,
    ArchivedGameSelected(ArchivedGame),
    BoardSizeSelected(BoardSize),
    CancelGame(Id),
    ChangeTheme(Theme),
    ConnectedTo(String),
    Coordinates(bool),
    DeleteAccount,
    EmailEveryone,
    EmailReset,
    EstimateScore,
    EstimateScoreConnected(mpsc::Sender<Tree>),
    EstimateScoreDisplay((Node, GenerateMove)),
    Exit,
    FocusPrevious,
    FocusNext,
    GameAccept(Id),
    GameCancel(Id),
    GameDecline(Id),
    GameJoin(Id),
    GameNew,
    GameResume(Id),
    GameSubmit,
    GameWatch(Id),
    HeatMap(bool),
    Leave,
    LeaveSoft,
    LocaleSelected(Locale),
    MyGamesOnly(bool),
    OpenUrl(String),
    PasswordChanged(String),
    PasswordSave(bool),
    PasswordShow(bool),
    PlayDraw,
    PlayDrawDecision(Draw),
    PlayMoveFrom(Vertex),
    PlayMoveTo(Vertex),
    PlayMoveRevert,
    PlayResign,
    PressEnter,
    PressA(bool),
    PressB(bool),
    PressC(bool),
    PressD(bool),
    PressE(bool),
    PressF(bool),
    PressG(bool),
    PressH(bool),
    PressI(bool),
    PressJ(bool),
    PressK(bool),
    PressL(bool),
    PressM(bool),
    PressN(bool),
    PressO(bool),
    PressP(bool),
    PressQ(bool),
    PressR(bool),
    PressS(bool),
    PressT(bool),
    PressU(bool),
    PressV(bool),
    PressW(bool),
    PressX(bool),
    PressY(bool),
    PressZ(bool),
    Press1,
    Press2,
    Press3,
    Press4,
    Press5,
    Press6,
    Press7,
    Press8,
    Press9,
    Press0,
    SoundMuted(bool),
    RatedSelected(bool),
    ResetPassword,
    ReviewGame,
    ReviewGameBackward,
    ReviewGameBackwardAll,
    ReviewGameChildNext,
    ReviewGameForward,
    ReviewGameForwardAll,
    RoleSelected(Role),
    ServerShutdown,
    StreamConnected(mpsc::Sender<String>),
    TcpDisconnect,
    TextChanged(String),
    TextEdit(text_editor::Action),
    TextReceived(String),
    TextSend,
    TextSendEmail,
    TextSendEmailCode,
    TextSendCreateAccount,
    TextSendLogin,
    Tick,
    Time(TimeEnum),
    Tournament,
    TournamentJoin,
    TournamentLeave,
    TournamentStart,
    TournamentDelete,
    TournamentTreeDelete,
    Users,
    UsersSortedBy(SortBy),
    WindowResized((f32, f32)),
}

#[derive(Clone, Debug)]
pub(crate) enum Move {
    From,
    To,
    Revert,
    None,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) enum Screen {
    AccountSettings,
    EmailEveryone,
    #[default]
    Login,
    Game,
    GameNew,
    GameReview,
    Games,
    Tournament,
    Users,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) enum Size {
    Tiny,
    TinyWide,
    #[default]
    Small,
    Medium,
    Large,
    Giant,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) enum SortBy {
    Name,
    #[default]
    Rating,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum State {
    Challenger,
    Creator,
    CreatorOnly,
    Spectator,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum Theme {
    #[default]
    Dark,
    Light,
    Tol,
}
