use std::mem;

use winapi::shared::windef::{HBRUSH, HDC};
use winapi::um::wingdi::{CreateSolidBrush, RGB};
use winapi::um::winuser::{BeginPaint, EndPaint, FillRect, FrameRect, PAINTSTRUCT};

use crate::common::{get_work_area, Rect};
use crate::window::Window;

const TILE_WIDTH: u32 = 48;
const TILE_HEIGHT: u32 = 48;

pub struct Grid {
    pub shift_down: bool,
    pub control_down: bool,
    pub selected_tile: Option<(usize, usize)>,
    pub hovered_tile: Option<(usize, usize)>,
    pub active_window: Option<Window>,
    pub grid_window: Option<Window>,
    pub previous_resize: Option<(Window, Rect)>,
    grid_margins: u8,
    zone_margins: u8,
    border_margins: u8,
    tiles: Vec<Vec<Tile>>, // tiles[row][column]
}

impl Default for Grid {
    fn default() -> Self {
        Grid {
            shift_down: false,
            control_down: false,
            selected_tile: None,
            hovered_tile: None,
            active_window: None,
            grid_window: None,
            previous_resize: None,
            grid_margins: 3,
            zone_margins: 10,
            border_margins: 10,
            tiles: vec![vec![Tile::default(); 2]; 2],
        }
    }
}

impl Grid {
    pub fn reset(&mut self) {
        self.control_down = false;
        self.shift_down = false;
        self.selected_tile = None;
        self.active_window = None;

        self.tiles.iter_mut().for_each(|row| {
            row.iter_mut().for_each(|tile| {
                tile.selected = false;
                tile.hovered = false;
            })
        });
    }

    pub fn dimensions(&self) -> (u32, u32) {
        let width = self.columns() as u32 * TILE_WIDTH
            + (self.columns() as u32 + 1) * self.grid_margins as u32;

        let height =
            self.rows() as u32 * TILE_HEIGHT + (self.rows() as u32 + 1) * self.grid_margins as u32;

        (width, height)
    }

    unsafe fn zone_area(&self, row: usize, column: usize) -> Rect {
        let mut work_area = get_work_area();
        work_area.width -= self.border_margins as i32 * 2;
        work_area.height -= self.border_margins as i32 * 2;

        let zone_width = (work_area.width - (self.columns() - 1) as i32 * self.zone_margins as i32)
            / self.columns() as i32;
        let zone_height = (work_area.height - (self.rows() - 1) as i32 * self.zone_margins as i32)
            / self.rows() as i32;

        let x =
            column as i32 * (work_area.width / self.columns() as i32) + self.border_margins as i32;
        let y = row as i32 * (work_area.height / self.rows() as i32) + self.border_margins as i32;

        Rect {
            x,
            y,
            width: zone_width,
            height: zone_height,
        }
    }

    fn rows(&self) -> usize {
        self.tiles.len()
    }

    fn columns(&self) -> usize {
        self.tiles[0].len()
    }

    pub fn add_row(&mut self) {
        self.tiles.push(vec![Tile::default(); self.columns()]);
    }

    pub fn add_column(&mut self) {
        for row in self.tiles.iter_mut() {
            row.push(Tile::default());
        }
    }

    pub fn remove_row(&mut self) {
        if self.rows() > 1 {
            self.tiles.pop();
        }
    }

    pub fn remove_column(&mut self) {
        if self.columns() > 1 {
            for row in self.tiles.iter_mut() {
                row.pop();
            }
        }
    }

    fn tile_area(&self, row: usize, column: usize) -> Rect {
        let x = column as i32 * TILE_WIDTH as i32 + (column as i32 + 1) * self.grid_margins as i32;

        let y = row as i32 * TILE_HEIGHT as i32 + (row as i32 + 1) * self.grid_margins as i32;

        Rect {
            x,
            y,
            width: TILE_WIDTH as i32,
            height: TILE_HEIGHT as i32,
        }
    }

    pub unsafe fn reposition(&self, mut window: Window) {
        let work_area = get_work_area();
        let dimensions = self.dimensions();

        let rect = Rect {
            x: work_area.width / 2 - dimensions.0 as i32 / 2,
            y: work_area.height / 2 - dimensions.1 as i32 / 2,
            width: dimensions.0 as i32,
            height: dimensions.1 as i32,
        };

        window.set_pos(rect, None);
    }

    /// Returns true if a change in highlighting occured
    pub unsafe fn highlight_tiles(&mut self, point: (i32, i32)) -> Option<Rect> {
        let original_tiles = self.tiles.clone();
        let mut hovered_rect = None;

        for row in 0..self.rows() {
            for column in 0..self.columns() {
                let tile_area = self.tile_area(row, column);

                if tile_area.contains_point(point) {
                    self.tiles[row][column].hovered = true;

                    self.hovered_tile = Some((row, column));
                    hovered_rect = Some(self.zone_area(row, column));
                } else {
                    self.tiles[row][column].hovered = false;
                }
            }
        }

        if let Some(rect) = self.shift_hover_and_calc_rect(true) {
            hovered_rect = Some(rect);
        }

        if original_tiles == self.tiles {
            None
        } else {
            hovered_rect
        }
    }

    unsafe fn shift_hover_and_calc_rect(&mut self, highlight: bool) -> Option<Rect> {
        if self.shift_down {
            if let Some(selected_tile) = self.selected_tile {
                if let Some(hovered_tile) = self.hovered_tile {
                    let selected_zone = self.zone_area(selected_tile.0, selected_tile.1);
                    let hovered_zone = self.zone_area(hovered_tile.0, hovered_tile.1);

                    let from_tile;
                    let to_tile;

                    let hovered_rect = if hovered_zone.x < selected_zone.x
                        && hovered_zone.y > selected_zone.y
                    {
                        from_tile = (selected_tile.0, hovered_tile.1);
                        to_tile = (hovered_tile.0, selected_tile.1);

                        let from_zone = self.zone_area(from_tile.0, from_tile.1);
                        let to_zone = self.zone_area(to_tile.0, to_tile.1);

                        Rect {
                            x: from_zone.x,
                            y: from_zone.y,
                            width: (to_zone.x + to_zone.width) - from_zone.x,
                            height: (to_zone.y + to_zone.height) - from_zone.y,
                        }
                    } else if hovered_zone.y < selected_zone.y && hovered_zone.x > selected_zone.x {
                        from_tile = (hovered_tile.0, selected_tile.1);
                        to_tile = (selected_tile.0, hovered_tile.1);

                        let from_zone = self.zone_area(from_tile.0, from_tile.1);
                        let to_zone = self.zone_area(to_tile.0, to_tile.1);

                        Rect {
                            x: from_zone.x,
                            y: from_zone.y,
                            width: (to_zone.x + to_zone.width) - from_zone.x,
                            height: (to_zone.y + to_zone.height) - from_zone.y,
                        }
                    } else if hovered_zone.x > selected_zone.x || hovered_zone.y > selected_zone.y {
                        from_tile = selected_tile;
                        to_tile = hovered_tile;

                        Rect {
                            x: selected_zone.x,
                            y: selected_zone.y,
                            width: (hovered_zone.x + hovered_zone.width) - selected_zone.x,
                            height: (hovered_zone.y + hovered_zone.height) - selected_zone.y,
                        }
                    } else {
                        from_tile = hovered_tile;
                        to_tile = selected_tile;

                        Rect {
                            x: hovered_zone.x,
                            y: hovered_zone.y,
                            width: (selected_zone.x + selected_zone.width) - hovered_zone.x,
                            height: (selected_zone.y + selected_zone.height) - hovered_zone.y,
                        }
                    };

                    if highlight {
                        for row in from_tile.0..=to_tile.0 {
                            for column in from_tile.1..=to_tile.1 {
                                self.tiles[row][column].hovered = true;
                            }
                        }
                    }

                    return Some(hovered_rect);
                }
            }
        }

        None
    }

    /// Returns true if a change in selected tile
    pub unsafe fn select_tile(&mut self, point: (i32, i32)) -> Option<Rect> {
        if let Some(shift_rect) = self.shift_hover_and_calc_rect(false) {
            return Some(shift_rect);
        }

        let previous_selected = self.selected_tile;

        for row in 0..self.rows() {
            for column in 0..self.columns() {
                let tile_area = self.tile_area(row, column);

                if tile_area.contains_point(point) {
                    self.tiles[row][column].selected = true;

                    self.selected_tile = Some((row, column));
                } else {
                    self.tiles[row][column].selected = false;
                }
            }
        }

        if previous_selected == self.selected_tile {
            None
        } else if let Some(selected_tile) = self.selected_tile {
            Some(self.zone_area(selected_tile.0, selected_tile.1))
        } else {
            None
        }
    }

    pub fn unhighlight_all_tiles(&mut self) {
        self.tiles
            .iter_mut()
            .for_each(|row| row.iter_mut().for_each(|tile| tile.hovered = false));
    }

    pub unsafe fn draw(&self, window: Window) {
        let mut paint: PAINTSTRUCT = mem::zeroed();
        //paint.fErase = 1;

        let hdc = BeginPaint(window.0, &mut paint);

        for row in 0..self.rows() {
            for column in 0..self.columns() {
                self.tiles[row][column].draw(hdc, self.tile_area(row, column));
            }
        }

        EndPaint(window.0, &paint);
    }
}

#[derive(Default, Clone, Copy, PartialEq)]
struct Tile {
    selected: bool,
    hovered: bool,
}

impl Tile {
    unsafe fn draw(self, hdc: HDC, area: Rect) {
        FillRect(hdc, &area.into(), self.fill_brush());
        FrameRect(hdc, &area.into(), CreateSolidBrush(RGB(0, 0, 0)));
    }

    unsafe fn fill_brush(self) -> HBRUSH {
        let color = if self.selected {
            RGB(0, 77, 128)
        } else if self.hovered {
            RGB(0, 100, 148)
        } else {
            RGB(
                (255.0 * (70.0 / 100.0)) as u8,
                (255.0 * (70.0 / 100.0)) as u8,
                (255.0 * (70.0 / 100.0)) as u8,
            )
        };

        CreateSolidBrush(color)
    }
}
