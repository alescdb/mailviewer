/* html_test.rs
 *
 * Copyright 2024 Alexandre Del Bigio
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

// TODO: still a lot to do here :)
#[cfg(test)]
mod tests {
  use mailviewer::Html;
  use std::{error::Error, fs};

  #[test]
  fn html() -> Result<(), Box<dyn Error>> {
    let html = Html::new(&fs::read_to_string("tests/test.html")?, true);
    let body = html.safe().to_lowercase();

    eprintln!("{}", &body);
    assert!(!body.contains("onblur="));
    assert!(!body.contains("onclick="));
    assert!(!body.contains("onchange="));
    assert!(!body.contains("style="));
    assert!(!body.contains("class="));

    assert!(!body.contains("<script>"));
    assert!(!body.contains("<meta>"));
    assert!(!body.contains("<audio>"));
    assert!(!body.contains("<video>"));
    assert!(!body.contains("<iframe>"));
    assert!(!body.contains("<link>"));
    assert!(!body.contains("<object>"));
    assert!(!body.contains("<embed>"));
    assert!(!body.contains("<applet>"));
    assert!(!body.contains("<form>"));
    
    assert!(body.contains(&mailviewer::html::CSS.to_lowercase()));

    Ok(())
  }
}
