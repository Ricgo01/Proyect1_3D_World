// main.rs
#![allow(unused_imports)]
#![allow(dead_code)]

mod line;
mod framebuffer;
mod maze;
mod caster;
mod player;
mod textures; // añadido

use line::line;
use maze::{Maze,load_maze};
use caster::{cast_ray, Intersect};
use framebuffer::Framebuffer;
use player::{Player, process_events};
use textures::TextureManager; // añadido

use raylib::prelude::*;
use std::thread;
use std::time::Duration;
use std::f32::consts::PI;

fn cell_to_color(cell: char) -> Color {
  match cell {
    '+' => {
      return Color::BLUEVIOLET;
    },
    '-' => {
      return Color::VIOLET;
    },
    '|' => {
      return Color::VIOLET;
    },
    'g' => {
      return Color::GREEN;
    },
    _ => {
      return Color::WHITE;
    },
  }
}

fn draw_cell(
  framebuffer: &mut Framebuffer,
  xo: usize,
  yo: usize,
  block_size: usize,
  cell: char,
) {
  if cell == ' ' {
    return;
  }
  let color = cell_to_color(cell);
  framebuffer.set_current_color(color);

  for x in xo..xo + block_size {
    for y in yo..yo + block_size {
      framebuffer.set_pixel(x as u32, y as u32);
    }
  }
}

pub fn render_maze(
  framebuffer: &mut Framebuffer,
  maze: &Maze,
  block_size: usize,
  player: &Player,
) {
  for (row_index, row) in maze.iter().enumerate() {
    for (col_index, &cell) in row.iter().enumerate() {
      let xo = col_index * block_size;
      let yo = row_index * block_size;
      draw_cell(framebuffer, xo, yo, block_size, cell);
    }
  }

  framebuffer.set_current_color(Color::WHITESMOKE);

  let num_rays = 5;
  for i in 0..num_rays {
    let current_ray = i as f32 / num_rays as f32; // current ray divided by total rays
    let a = player.a - (player.fov / 2.0) + (player.fov * current_ray);
    cast_ray(framebuffer, &maze, &player, a, block_size, true);
  }
}

fn render_world(
  framebuffer: &mut Framebuffer,
  maze: &Maze,
  block_size: usize,
  player: &Player,
  tex: &TextureManager,
) {
  let num_rays = framebuffer.width as usize;
  let half_h = framebuffer.height as i32 / 2;
  let proj_plane = (num_rays as f32) / (2.0 * (player.fov * 0.5).tan());

  for sx in 0..num_rays {
    let cam_x = (2.0 * sx as f32 / num_rays as f32) - 1.0; // [-1,1]
    let ray_angle = player.a + cam_x * (player.fov * 0.5);
    let inter = cast_ray(framebuffer, maze, player, ray_angle, block_size, false);
    if inter.impact == ' ' || inter.distance <= 0.0 { continue; }

    // inter.distance ya es perpendicular (DDA)
    let dist = inter.distance;
    let wall_h = (block_size as f32 * proj_plane / dist) as i32;
    let mut top = half_h - wall_h / 2;
    let mut bottom = half_h + wall_h / 2;
    if top < 0 { top = 0; }
    if bottom >= framebuffer.height as i32 { bottom = framebuffer.height as i32 - 1; }

    let (tw, th) = tex.get_size(inter.impact);
    let mut tx = (inter.wall_x * tw as f32) as u32;
    // Invertir para mantener orientación según dirección
    if inter.side == 0 && ray_angle.cos() > 0.0 { tx = tw.saturating_sub(1) - tx; }
    if inter.side == 1 && ray_angle.sin() < 0.0 { tx = tw.saturating_sub(1) - tx; }

    let column_h = (bottom - top).max(1) as f32;
    for sy in top..=bottom {
      let rel = (sy - top) as f32 / column_h;
      let ty = (rel * th as f32) as u32;
      let mut color = tex.sample(inter.impact, tx, ty);
      let base = (1.0 / (1.0 + dist * 0.002)).clamp(0.2, 1.0);
      let side_factor = if inter.side == 1 { 0.75 } else { 1.0 }; // caras horizontales más oscuras
      let shade = (base * side_factor).clamp(0.15, 1.0);
      color.r = (color.r as f32 * shade) as u8;
      color.g = (color.g as f32 * shade) as u8;
      color.b = (color.b as f32 * shade) as u8;
      framebuffer.set_current_color(color);
      framebuffer.set_pixel(sx as u32, sy as u32);
    }
  }
}

fn main() {
  let window_width = 1300;
  let window_height = 900;
  let block_size = 64; // Alinear con texturas

  let (mut window, raylib_thread) = raylib::init()
    .size(window_width, window_height)
    .title("Raycaster Example")
    .log_level(TraceLogLevel::LOG_WARNING)
    .build();

  window.disable_cursor();

  let mut framebuffer = Framebuffer::new(window_width as u32, window_height as u32);
  framebuffer.set_background_color(Color::new(50, 50, 100, 255));

  let maze = load_maze("maze.txt");

  let mut player = Player {
    pos: Vector2::new(2.5 * block_size as f32, 2.5 * block_size as f32),
    a: 0.0,
    fov: PI / 3.0,
  };

  // Cargar texturas una sola vez
  let mut tex_manager = TextureManager::new();
  tex_manager.load_defaults();

  // Persistir modo fuera del loop
  let mut mode = "3D";

  while !window.window_should_close() {
    let dt = window.get_frame_time();
    process_events(&mut player, &window, dt, &maze, block_size);

    if window.is_key_pressed(KeyboardKey::KEY_M) {
      mode = if mode == "2D" { "3D" } else { "2D" };
    }

    framebuffer.clear();

    if mode == "2D" {
      render_maze(&mut framebuffer, &maze, block_size, &player);
    } else {
      render_world(&mut framebuffer, &maze, block_size, &player, &tex_manager);
    }

    framebuffer.swap_buffers(&mut window, &raylib_thread);
    thread::sleep(Duration::from_millis(16));
  }
}
