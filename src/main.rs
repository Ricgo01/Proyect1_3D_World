// main.rs
#![allow(unused_imports)]
#![allow(dead_code)]

mod line;
mod framebuffer;
mod maze;
mod caster;
mod player;
mod textures;
mod sprites;

use line::line;
use maze::{Maze,load_maze};
use caster::{cast_ray, Intersect};
use framebuffer::Framebuffer;
use player::{Player, process_events};
use textures::TextureManager;
use sprites::{draw_sprites, Enemy};

use raylib::{ffi::RL_TEXTURE_MIN_FILTER, prelude::*};
use std::thread;
use std::time::Duration;
use std::f32::consts::PI;
use raylib::core::audio::{RaylibAudio, Sound};

#[derive(Copy, Clone, PartialEq)]
enum GameState {
    Start,
    Playing,
}

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

const PLAYER_MINIMAP_COLOR: Color = Color::new(255, 80, 0, 255);

fn render_minimap(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    block_size: usize,
    player: &Player,
    origin_x: u32,
    origin_y: u32,
    cell_px: u32,
) {
    for (row_i, row) in maze.iter().enumerate() {
        for (col_i, &cell) in row.iter().enumerate() {
            if cell == ' ' { continue; }
            let color = cell_to_color(cell);
            for py in 0..cell_px {
                for px in 0..cell_px {
                    framebuffer.set_pixel_color(
                        origin_x + col_i as u32 * cell_px + px,
                        origin_y + row_i as u32 * cell_px + py,
                        color
                    );
                }
            }
        }
    }

    let px_cell = player.pos.x / block_size as f32;
    let py_cell = player.pos.y / block_size as f32;
    let pxm = origin_x + (px_cell * cell_px as f32) as u32;
    let pym = origin_y + (py_cell * cell_px as f32) as u32;
    framebuffer.set_pixel_color(pxm,   pym,   PLAYER_MINIMAP_COLOR);
    framebuffer.set_pixel_color(pxm+1, pym,   PLAYER_MINIMAP_COLOR);
    framebuffer.set_pixel_color(pxm,   pym+1, PLAYER_MINIMAP_COLOR);
}

fn render_world(
  framebuffer: &mut Framebuffer,
  maze: &Maze,
  block_size: usize,
  player: &Player,
  tex: &TextureManager,
  depth_buffer: &mut [f32],
) {
  let num_rays = framebuffer.width as usize;
  let half_h = framebuffer.height as i32 / 2;
  let proj_plane = (num_rays as f32) / (2.0 * (player.fov * 0.5).tan());

  // Cielo
  for y in 0..half_h { // y entero
    let v = y as f32 / half_h as f32; // 0..1
    for x in 0..framebuffer.width as u32 {
      let u = (player.a / (2.0 * PI)) + (x as f32 / framebuffer.width as f32);
      let col = tex.sample_sky(u, v);
      framebuffer.set_current_color(col);
      framebuffer.set_pixel(x, y as u32);
    }
  }

  // Suelo (floor casting simplificado)
  let dir_left_angle = player.a - player.fov * 0.5;
  let dir_right_angle = player.a + player.fov * 0.5;
  let dir_left = Vector2::new(dir_left_angle.cos(), dir_left_angle.sin());
  let dir_right = Vector2::new(dir_right_angle.cos(), dir_right_angle.sin());
  let px = player.pos.x / block_size as f32;
  let py = player.pos.y / block_size as f32;

  for sy in half_h..(framebuffer.height as i32) {
    let p = (sy - half_h) as f32;
    if p < 1.0 { continue; }
    let row_dist = (0.5 * framebuffer.height as f32) / p;

    // Pre-shade por fila
    let shade = (1.0 / (1.0 + row_dist * 0.15)).clamp(0.15, 1.0);

    let step_x = row_dist * (dir_right.x - dir_left.x) / num_rays as f32;
    let step_y = row_dist * (dir_right.y - dir_left.y) / num_rays as f32;
    let mut floor_x = px + row_dist * dir_left.x;
    let mut floor_y = py + row_dist * dir_left.y;

    for sx in 0..num_rays {
      let u = floor_x.fract();
      let v = floor_y.fract();
      let mut col = tex.sample_ground(u, v);
      col.r = (col.r as f32 * shade) as u8;
      col.g = (col.g as f32 * shade) as u8;
      col.b = (col.b as f32 * shade) as u8;
      framebuffer.set_pixel_color(sx as u32, sy as u32, col);
      floor_x += step_x;
      floor_y += step_y;
    }
  }

  // Muros
  for sx in 0..num_rays {
    let cam_x = (2.0 * sx as f32 / num_rays as f32) - 1.0; // [-1,1]
    let ray_angle = player.a + cam_x * (player.fov * 0.5);
    let inter = cast_ray(framebuffer, maze, player, ray_angle, block_size, false);
    if inter.impact == ' ' || inter.distance <= 0.0 { 
      depth_buffer[sx] = f32::INFINITY;
      continue; 
    }

    depth_buffer[sx] = inter.distance;
    let dist = inter.distance;
    let wall_h = (block_size as f32 * proj_plane / dist) as i32;
    let mut top = half_h - wall_h / 2;
    let mut bottom = half_h + wall_h / 2;
    if top < 0 { top = 0; }
    if bottom >= framebuffer.height as i32 { bottom = framebuffer.height as i32 - 1; }

    let (tw, th) = tex.get_size(inter.impact);
    let mut tx = (inter.wall_x * tw as f32) as u32;
    if inter.side == 0 && ray_angle.cos() > 0.0 { tx = tw.saturating_sub(1) - tx; }
    if inter.side == 1 && ray_angle.sin() < 0.0 { tx = tw.saturating_sub(1) - tx; }

    let base = (1.0 / (1.0 + dist * 0.002)).clamp(0.2, 1.0);
    let side_factor = if inter.side == 1 { 0.75 } else { 1.0 };
    let shade = (base * side_factor).clamp(0.15, 1.0);

    let column_h = (bottom - top).max(1) as f32;
    let ty_step = th as f32 / column_h;
    let mut ty_f = 0.0;

    for sy in top..=bottom {
      let ty = ty_f as u32;
      ty_f += ty_step;

      let mut c = tex.sample(inter.impact, tx, ty);
      // Aplicar shade
      c.r = (c.r as f32 * shade) as u8;
      c.g = (c.g as f32 * shade) as u8;
      c.b = (c.b as f32 * shade) as u8;

      framebuffer.set_pixel_color(sx as u32, sy as u32, c);
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

  let mut game_start = GameState::Start;

  let audio = match RaylibAudio::init_audio_device() {
        Ok(dev) => dev,
        Err(e) => {
            eprintln!("No se pudo inicializar el audio: {e}");
            return;
        }
  };


  let mut footstep_sound: Sound = match audio.new_sound("assets/sounds/steps.wav") {
    Ok(snd) => snd,
    Err(e) => {
        eprintln!("Failed to load footstep sound: {e}");
        return;
    }
  };

  let taylor: Sound = match audio.new_sound("assets/sounds/tay.wav") {
    Ok(snd) => snd,
    Err(e) => {
        eprintln!("Failed to load footstep sound: {e}");
        return;
    }
  };



  let mut framebuffer = Framebuffer::new(window_width as u32, window_height as u32);
  framebuffer.set_background_color(Color::new(50, 50, 100, 255));

  let maze = load_maze("maze.txt");

  let mut player = Player {
    pos: Vector2::new(2.5 * block_size as f32, 2.5 * block_size as f32),
    a: 0.0,
    fov: PI / 3.0,
  };

   let mut enemies = vec![
      Enemy::new(5.5 * block_size as f32, 1.5 * block_size as f32, 'e'),
      Enemy::new(3.5 * block_size as f32, 2.5 * block_size as f32, 'f'),
      Enemy::new(4.5 * block_size as f32, 1.5 * block_size as f32, 'k'),
      Enemy::new(6.5 * block_size as f32, 1.5 * block_size as f32, 'p'),
      
  ];

  let mut depth_buffer = vec![0.0f32; window_width as usize];
  let mut tex_manager = TextureManager::new();
  tex_manager.load_defaults();

  while !window.window_should_close() {

    if game_start == GameState::Start {
          if window.is_key_pressed(KeyboardKey::KEY_ENTER) {
              game_start= GameState::Playing;
          } else {
              let mut d = window.begin_drawing(&raylib_thread);
              d.clear_background(Color::BLACK);
              d.draw_text("RAYCASTER", 60, 80, 70, Color::WHITE);
              d.draw_text("Presiona ENTER para empezar", 60, 200, 30, Color::RAYWHITE);
              d.draw_text("WASD / Flechas = mover, ESC = salir", 60, 250, 20, Color::GRAY);
              d.draw_text("Version demo", 60, 290, 20, Color::DARKGRAY);
              continue;
          }
      }
    
    let dt = window.get_frame_time();

    process_events(&mut player, &window, dt, &maze, block_size, Some(&mut footstep_sound));
    
    let tay_proximity: f32 = 200.0;
    let mut any_p_in_range = false;

    for enemy in &enemies {
      if enemy.id == 'p' {
          let dx = enemy.pos.x - player.pos.x;
          let dy = enemy.pos.y - player.pos.y;
          let dist_sq = dx*dx + dy*dy;

          if dist_sq < tay_proximity * tay_proximity {
              any_p_in_range = true;
              break;
          }
        }
    }

    if any_p_in_range {
        if !taylor.is_playing() {
            taylor.play();
        }
    } else {
        if taylor.is_playing() {
            taylor.stop();
        }
    }
    framebuffer.clear();

    let proj_plane = (framebuffer.width as f32) / (2.0 * (player.fov * 0.5).tan());


    render_world(&mut framebuffer, &maze, block_size, &player, &tex_manager, &mut depth_buffer);

    draw_sprites(
        &mut framebuffer,
        &player,
        &mut enemies,
        &tex_manager,
        &depth_buffer,
        proj_plane,
        block_size,
    );

    let cell_px = 16;
    let mini_h = maze.len() as u32 * cell_px;
    let margin = 8;
    let ox = margin;
    let oy = framebuffer.height - mini_h - margin;
    render_minimap(&mut framebuffer, &maze, block_size, &player, ox, oy, cell_px);

    framebuffer.swap_buffers(&mut window, &raylib_thread, true);
    thread::sleep(Duration::from_millis(16));
  }
}
