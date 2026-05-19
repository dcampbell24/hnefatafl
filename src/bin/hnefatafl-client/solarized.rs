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
//
// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 David Campbell <david@hnefatafl.org>

/*
SOLARIZED HEX     16/8 TERMCOL  XTERM/HEX   L*A*B      RGB         HSB
--------- ------- ---- -------  ----------- ---------- ----------- -----------
base03    #002b36  8/4 brblack  234 #1c1c1c 15 -12 -12   0  43  54 193 100  21
base02    #073642  0/4 black    235 #262626 20 -12 -12   7  54  66 192  90  26
base01    #586e75 10/7 brgreen  240 #585858 45 -07 -07  88 110 117 194  25  46
base00    #657b83 11/7 bryellow 241 #626262 50 -07 -07 101 123 131 195  23  51

base0     #839496 12/6 brblue   244 #808080 60 -06 -03 131 148 150 186  13  59
base1     #93a1a1 14/4 brcyan   245 #8a8a8a 65 -05 -02 147 161 161 180   9  63
base2     #eee8d5  7/7 white    254 #e4e4e4 92 -00  10 238 232 213  44  11  93
base3     #fdf6e3 15/7 brwhite  230 #ffffd7 97  00  10 253 246 227  44  10  99

yellow    #b58900  3/3 yellow   136 #af8700 60  10  65 181 137   0  45 100  71
orange    #cb4b16  9/3 brred    166 #d75f00 50  50  55 203  75  22  18  89  80
red       #dc322f  1/1 red      160 #d70000 50  65  45 220  50  47   1  79  86
magenta   #d33682  5/5 magenta  125 #af005f 50  65 -05 211  54 130 331  74  83
violet    #6c71c4 13/5 brmagenta 61 #5f5faf 50  15 -45 108 113 196 237  45  77
blue      #268bd2  4/4 blue      33 #0087ff 55 -10 -45  38 139 210 205  82  82
cyan      #2aa198  6/6 cyan      37 #00afaf 60 -35 -05  42 161 152 175  74  63
green     #859900  2/2 green     64 #5f8700 60 -20  65 133 153   0  68 100  60
*/

use iced::Color;

#[must_use]
pub fn _base03() -> Color {
    Color::from_rgb8(0, 43, 54)
}

#[must_use]
pub fn _base02() -> Color {
    Color::from_rgb8(7, 54, 66)
}

#[must_use]
pub fn _base01() -> Color {
    Color::from_rgb8(88, 110, 117)
}

#[must_use]
pub fn _base00() -> Color {
    Color::from_rgb8(101, 123, 131)
}

#[must_use]
pub fn _base0() -> Color {
    Color::from_rgb8(131, 148, 150)
}

#[must_use]
pub fn _base1() -> Color {
    Color::from_rgb8(147, 161, 161)
}

#[must_use]
pub fn _base2() -> Color {
    Color::from_rgb8(238, 232, 213)
}

#[must_use]
pub fn _base3() -> Color {
    Color::from_rgb8(253, 246, 227)
}

#[must_use]
pub fn yellow() -> Color {
    Color::from_rgb8(181, 137, 0)
}

#[must_use]
pub fn _orange() -> Color {
    Color::from_rgb8(203, 75, 22)
}

#[must_use]
pub fn red() -> Color {
    Color::from_rgb8(220, 50, 47)
}

#[must_use]
pub fn _magenta() -> Color {
    Color::from_rgb8(211, 54, 130)
}

#[must_use]
pub fn _violet() -> Color {
    Color::from_rgb8(108, 113, 196)
}

#[must_use]
pub fn blue() -> Color {
    Color::from_rgb8(38, 139, 210)
}

#[must_use]
pub fn _cyan() -> Color {
    Color::from_rgb8(42, 161, 152)
}

#[must_use]
pub fn green() -> Color {
    Color::from_rgb8(133, 153, 0)
}
