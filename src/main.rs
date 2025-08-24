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
mod levels; 

use line::line;
use maze::{Maze,load_maze};
use caster::{cast_ray, Intersect};
use framebuffer::Framebuffer;
use player::{Player, process_events};
use textures::TextureManager;
use sprites::{draw_sprites, Enemy};
use levels::{GameState, LevelDef, LEVELS, load_level};

use raylib::{ffi::RL_TEXTURE_MIN_FILTER, prelude::*};
use std::thread;
use std::time::Duration;
use std::f32::consts::PI;
use raylib::core::audio::{RaylibAudio, Sound};


// function that converts a maze cell to a color for minimap rendering
fn cell_to_color(cell: char) -> Color {
  match cell {
    '+' => {
      return Color::LAVENDER;
    },
    '-' => {
      return Color::LIGHTBLUE;
    },
    '|' => {
      return Color::WHITE;
    },
    'g' => {
      return Color::GREEN;
    },
    _ => {
      return Color::WHITE;
    },
  }
}

//function that draws a cell of the maze on the framebuffer

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



//this renders the minimap on to the framebuffer

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

    // Draw player as a small square
    for dy in -5..=5 {
        for dx in -5..=5 {
            let x = pxm as i32 + dx;
            let y = pym as i32 + dy;
            if x >= 0 && y >= 0 {
                framebuffer.set_pixel_color(x as u32, y as u32, Color::VIOLET);
            }
        }
    }

    // Draw direction line
    let dir_len = 12;
    for i in 0..dir_len {
        let x = pxm as i32 + (player.a.cos() * i as f32) as i32;
        let y = pym as i32 + (player.a.sin() * i as f32) as i32;
        if x >= 0 && y >= 0 {
            framebuffer.set_pixel_color(x as u32, y as u32, Color::YELLOW);
        }
    }
    framebuffer.set_pixel_color(pxm,   pym,   Color::VIOLET);
    framebuffer.set_pixel_color(pxm+1, pym,   Color::VIOLET);
    framebuffer.set_pixel_color(pxm,   pym+1, Color::VIOLET);
}

//This function renders the 3d world to the framebuffer and check the colisions
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
  let mut ray_pre: Vec<(f32, f32)> = Vec::with_capacity(num_rays);

  //flashlight parameters
  let flashlight_cone_half = PI / 9.0; 
  let flashlight_max_dist = 7.0 * block_size as f32;
  let flashlight_max_dist_sq = flashlight_max_dist * flashlight_max_dist;
  let ambient = 0.05;
  let inv_cone = 1.0 / flashlight_cone_half;
  let inv_flash = 1.0 / flashlight_max_dist;
  let hotspot_scale = 1.0 / (flashlight_max_dist * 0.25);

  ray_pre.clear();
  for sx in 0..num_rays {
    let cam_x = (2.0 * sx as f32 / num_rays as f32) - 1.0;
    let ray_angle = player.a + cam_x * (player.fov * 0.5);
    let mut ang_diff = ray_angle - player.a;
    if ang_diff > PI { ang_diff -= 2.0*PI; }
    if ang_diff < -PI { ang_diff += 2.0*PI; }
    let ang_factor_lin = (1.0 - (ang_diff.abs() * inv_cone)).clamp(0.0,1.0);
    let ang_factor = ang_factor_lin * ang_factor_lin.sqrt();
    ray_pre.push((ray_angle, ang_factor));
  }

  // Render the sky
  for y in 0..half_h {
    let v = y as f32 / half_h as f32;
    for x in 0..framebuffer.width as u32 {
      let u = (player.a / (2.0 * PI)) + (x as f32 / framebuffer.width as f32);
      let col = tex.sample_sky(u, v);
      framebuffer.set_current_color(col);
      framebuffer.set_pixel(x, y as u32);
    }
  }

  
  let dir_left_angle = player.a - player.fov * 0.5;
  let dir_right_angle = player.a + player.fov * 0.5;
  let dir_left = Vector2::new(dir_left_angle.cos(), dir_left_angle.sin());
  let dir_right = Vector2::new(dir_right_angle.cos(), dir_right_angle.sin());
  let px = player.pos.x / block_size as f32;
  let py = player.pos.y / block_size as f32;


  let floor_step: usize = 2;

  //render the floor
  for sy in (half_h..(framebuffer.height as i32)).step_by(floor_step) {
    let p = (sy - half_h) as f32;
    if p < 1.0 { continue; }
    let row_dist = (0.5 * framebuffer.height as f32) / p;

    let shade_row = (1.0 / (1.0 + row_dist * 0.15)).clamp(0.05, 1.0);

    let step_x = row_dist * (dir_right.x - dir_left.x) / num_rays as f32;
    let step_y = row_dist * (dir_right.y - dir_left.y) / num_rays as f32;
    let mut floor_x = px + row_dist * dir_left.x;
    let mut floor_y = py + row_dist * dir_left.y;

    let floor_far = (row_dist * block_size as f32) > flashlight_max_dist;

    for sx in 0..num_rays {
      let u = floor_x.fract();
      let v = floor_y.fract();
      let mut col = tex.sample_ground(u, v);

      if floor_far {
        col.r = (col.r as f32 * shade_row * ambient) as u8;
        col.g = (col.g as f32 * shade_row * ambient) as u8;
        col.b = (col.b as f32 * shade_row * ambient) as u8;
        framebuffer.set_pixel_color(sx as u32, sy as u32, col);
        if floor_step > 1 {
          let sy2 = sy + 1;
          if sy2 < framebuffer.height as i32 { framebuffer.set_pixel_color(sx as u32, sy2 as u32, col); }
        }
        floor_x += step_x; floor_y += step_y; continue;
      }

      let world_x = floor_x * block_size as f32;
      let world_y = floor_y * block_size as f32;
      let dxw = world_x - player.pos.x;
      let dyw = world_y - player.pos.y;
      let dist_w_sq = dxw*dxw + dyw*dyw;
      if dist_w_sq > flashlight_max_dist_sq {
        col.r = (col.r as f32 * shade_row * ambient) as u8;
        col.g = (col.g as f32 * shade_row * ambient) as u8;
        col.b = (col.b as f32 * shade_row * ambient) as u8;
        framebuffer.set_pixel_color(sx as u32, sy as u32, col);
        if floor_step > 1 { let sy2 = sy + 1; if sy2 < framebuffer.height as i32 { framebuffer.set_pixel_color(sx as u32, sy2 as u32, col); } }
        floor_x += step_x; floor_y += step_y; continue;
      }
      let dist_w = dist_w_sq.sqrt();
      let ang_w = dyw.atan2(dxw) - player.a;
      let mut ang_norm = ang_w;
      if ang_norm > PI { ang_norm -= 2.0*PI; }
      if ang_norm < -PI { ang_norm += 2.0*PI; }
      let ang_factor_lin = (1.0 - (ang_norm.abs() * inv_cone)).clamp(0.0,1.0);
      let ang_factor = ang_factor_lin * ang_factor_lin.sqrt();
      let dist_factor_lin = (1.0 - (dist_w * inv_flash)).clamp(0.0,1.0);
      let sqrt_d = dist_factor_lin.sqrt();
      let dist_factor = sqrt_d + (dist_factor_lin - sqrt_d) * 0.4;
      let hw = 1.0 - (dist_w * hotspot_scale).clamp(0.0,1.0);
      let hw2 = hw * hw;
      let hotspot = hw2 * (0.86 + 0.14 * hw) * ang_factor;
      let core_base = ang_factor * dist_factor;
      let s1 = core_base.sqrt();
      let s2 = s1.sqrt();
      let core = core_base * s2;
      let mut light = ambient + (1.0 - ambient) * core + 0.35 * hotspot;
      if light > 1.0 { light = 1.0; }

      col.r = (col.r as f32 * shade_row * light) as u8;
      col.g = (col.g as f32 * shade_row * light) as u8;
      col.b = (col.b as f32 * shade_row * light) as u8;
      framebuffer.set_pixel_color(sx as u32, sy as u32, col);
      if floor_step > 1 {
        let sy2 = sy + 1;
        if sy2 < framebuffer.height as i32 { framebuffer.set_pixel_color(sx as u32, sy2 as u32, col); }
      }

      floor_x += step_x;
      floor_y += step_y;
    }
  }

  //Render the walls
  for sx in 0..num_rays {
    let (ray_angle, ang_factor) = ray_pre[sx];
    let inter = cast_ray(framebuffer, maze, player, ray_angle, block_size, false);
    if inter.impact == ' ' || inter.distance <= 0.0 { depth_buffer[sx] = f32::INFINITY; continue; }
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
    let mut shade = (base * side_factor).clamp(0.15, 1.0);

    let dist_factor_lin = (1.0 - (dist * inv_flash)).clamp(0.0,1.0);
    let sqrt_d = dist_factor_lin.sqrt();
    let dist_factor = sqrt_d + (dist_factor_lin - sqrt_d) * 0.4;
    let mut light;
    if dist > flashlight_max_dist {
      light = ambient;
    } else {
      let hw = 1.0 - (dist * hotspot_scale).clamp(0.0,1.0);
      let hw2 = hw * hw;
      let hotspot = hw2 * (0.86 + 0.14 * hw) * ang_factor;
      let core_base = ang_factor * dist_factor;
      let s1 = core_base.sqrt();
      let s2 = s1.sqrt();
      let core = core_base * s2;
      light = ambient + (1.0 - ambient) * core + 0.35 * hotspot;
      if light > 1.0 { light = 1.0; }
    }
    shade = (shade * light).clamp(ambient,1.0);

    let column_h = (bottom - top).max(1) as f32;
    let ty_step = th as f32 / column_h;
    let mut ty_f = 0.0;

    for sy in top..=bottom {
      let ty = ty_f as u32;
      ty_f += ty_step;

      let mut c = tex.sample(inter.impact, tx, ty);
      c.r = (c.r as f32 * shade) as u8;
      c.g = (c.g as f32 * shade) as u8;
      c.b = (c.b as f32 * shade) as u8;

      framebuffer.set_pixel_color(sx as u32, sy as u32, c);
    }
  }
  
}


//Main function that manages the main render loop and the game logic

fn main() {
  let window_width = 1300;
  let window_height = 900;
  let block_size = 64;

  //create the window

  let (mut window, raylib_thread) = raylib::init()
    .size(window_width, window_height)
    .title("Raycaster Example")
    .log_level(TraceLogLevel::LOG_WARNING)
    .build();

  window.disable_cursor();

  let mut game_state = GameState::Start;
  let mut selected_level: usize = 0;

  //initialize the audio

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

  let bg_music: Sound = match audio.new_sound("assets/sounds/scary.mp3") {
      Ok(snd) => snd,
      Err(e) => { eprintln!("No se pudo cargar música fondo: {e}"); return; }
  };

  //create the framebuffer

  let mut framebuffer = Framebuffer::new(window_width as u32, window_height as u32);
  framebuffer.set_background_color(Color::new(50, 50, 100, 255));


  let mut maze: Option<Maze> = None;
  let mut enemies: Vec<Enemy> = Vec::new();
  let mut depth_buffer = vec![0.0f32; window_width as usize];
  let mut tex_manager = TextureManager::new();
  tex_manager.load_defaults();

  let mut player = Player { pos: Vector2::new(0.0, 0.0), a: 0.0, fov: PI / 3.0 };
  let mut has_key: bool = false;
  let _just_won: bool = false;

  //Main render loop

  while !window.window_should_close() {

    //if the game state is start, we show the main menu

    if game_state == GameState::Start {
        if window.is_key_pressed(KeyboardKey::KEY_DOWN) {
            selected_level = (selected_level + 1) % LEVELS.len();
        }
        if window.is_key_pressed(KeyboardKey::KEY_UP) {
            selected_level = (selected_level + LEVELS.len() - 1) % LEVELS.len();
        }
        if window.is_key_pressed(KeyboardKey::KEY_ENTER) {
            let def = &LEVELS[selected_level];
            let (m, es, start) = load_level(def, block_size);
            maze = Some(m);
            enemies = es;
            player.pos = Vector2::new(start.0 * block_size as f32, start.1 * block_size as f32);
            player.a = start.2;
            game_state = GameState::Playing;
        }



        let mut d = window.begin_drawing(&raylib_thread);
        d.clear_background(Color::BLACK);
        d.draw_text("SCARY CLUB", 60, 40, 70, Color::WHITE);
        d.draw_text("Ayuda al abuelo a encontrar la llave para entrar a su casa", 70, 130, 30, Color::WHITE);
        d.draw_text("y evita al malvado en una noche obscura", 70, 150, 30, Color::WHITE);
        let (tw, th) = tex_manager.get_size('c');
        if tw > 1 && th > 1 {
            let scale = (window_height as f32 * 0.5) / th as f32;
            let draw_w = (tw as f32 * scale) as u32;
            let draw_h = (th as f32 * scale) as u32;
            let x0 = (window_width as u32 / 2).saturating_sub(draw_w / 2);
            let y0 = (window_height as u32 / 2).saturating_sub(draw_h / 2);
            for dy in 0..draw_h {
                let ty = (dy as f32 / draw_h as f32 * th as f32) as u32;
                for dx in 0..draw_w {
                    let tx = (dx as f32 / draw_w as f32 * tw as f32) as u32;
                    let c = tex_manager.sample('c', tx, ty);
                    if (c.r, c.g, c.b) != (255, 0, 255) {
                        d.draw_pixel((x0 + dx) as i32, (y0 + dy) as i32, c);
                    }
                }
            }
        }
        d.draw_text("Selecciona nivel (UP/DOWN) y ENTER", 60, 700, 24, Color::RAYWHITE);
        for (i, def) in LEVELS.iter().enumerate() {
            let col = if i == selected_level { Color::YELLOW } else { Color::GRAY };
            d.draw_text(def.name, 80, 750 + (i as i32) * 32, 28, col);
        }
        d.draw_text("ESC para salir", 60, 400 + (LEVELS.len() as i32) * 32 + 20, 20, Color::DARKGRAY);
        continue;
    }

    //if game state is game over, we show a game over screen on the framebuffer

    if game_state == GameState::GameOver {
        if window.is_key_pressed(KeyboardKey::KEY_ENTER) || window.is_key_pressed(KeyboardKey::KEY_M) {
            game_state = GameState::Start;
            has_key = false;
            continue;
        }
        let mut d = window.begin_drawing(&raylib_thread);
        d.clear_background(Color::BLACK);
        d.draw_text("GAME OVER", 400, 70, 70, Color::RED);
        d.draw_text("ENTER: reintentar nivel / M: volver al menú", 300, 160, 30, Color::RAYWHITE);
        let (tw, th) = tex_manager.get_size('o');
        if tw > 1 && th > 1 {
            let scale = (window_height as f32 * 0.5) / th as f32;
            let draw_w = (tw as f32 * scale) as u32;
            let draw_h = (th as f32 * scale) as u32;
            let x0 = (window_width as u32 / 2).saturating_sub(draw_w / 2);
            let y0 = (window_height as u32 / 2).saturating_sub(draw_h / 2);
            for dy in 0..draw_h {
                let ty = (dy as f32 / draw_h as f32 * th as f32) as u32;
                for dx in 0..draw_w {
                    let tx = (dx as f32 / draw_w as f32 * tw as f32) as u32;
                    let c = tex_manager.sample('o', tx, ty);
                    if (c.r, c.g, c.b) != (255, 0, 255) {
                        d.draw_pixel((x0 + dx) as i32, (y0 + dy) as i32, c);
                    }
                }
            }
        }
        continue;
    }

    //if game state is win, we show a win screen on the framebuffer

    if game_state == GameState::Win {
        if window.is_key_pressed(KeyboardKey::KEY_ENTER) {
            selected_level = (selected_level + 1) % LEVELS.len();
            let def = &LEVELS[selected_level];
            let (m, es, start) = load_level(def, block_size);
            maze = Some(m);
            enemies = es;
            player.pos = Vector2::new(start.0 * block_size as f32, start.1 * block_size as f32);
            player.a = start.2;
            has_key = false;
            game_state = GameState::Playing;
            continue;
        }
        if window.is_key_pressed(KeyboardKey::KEY_M) {
            game_state = GameState::Start;
            has_key = false;
            continue;
        }

        let mut d = window.begin_drawing(&raylib_thread);
        d.clear_background(Color::WHITE);
        d.draw_text("NIVEL COMPLETADO", 350, 100, 60, Color::GREEN);
        d.draw_text("ENTER: siguiente nivel", 500, 200, 30, Color::BLACK);
        d.draw_text("M: menú", 500, 240, 30, Color::BLACK);

        let (tw, th) = tex_manager.get_size('w');
        if tw > 1 && th > 1 {
            let scale = (window_height as f32 * 0.5) / th as f32;
            let draw_w = (tw as f32 * scale) as u32;
            let draw_h = (th as f32 * scale) as u32;
            let x0 = (window_width as u32 / 2).saturating_sub(draw_w / 2);
            let y0 = (window_height as u32 / 2).saturating_sub(draw_h / 4);
            for dy in 0..draw_h {
                let ty = (dy as f32 / draw_h as f32 * th as f32) as u32;
                for dx in 0..draw_w {
                    let tx = (dx as f32 / draw_w as f32 * tw as f32) as u32;
                    let c = tex_manager.sample('w', tx, ty);
                    if (c.r, c.g, c.b) != (255, 0, 255) {
                        d.draw_pixel((x0 + dx) as i32, (y0 + dy) as i32, c);
                    }
                }
            }
        }
        continue;
    }





    let maze_ref = if let Some(m) = maze.as_ref() { m } else { continue; };

    if !bg_music.is_playing() {
        bg_music.play();
        bg_music.set_volume(0.3);
    }

    let dt = window.get_frame_time();
    process_events(&mut player, &window, dt, maze_ref, block_size, Some(&mut footstep_sound));

    // The bad guy can follow the player in a certain radius
    const ENEMY_CHASE_SPEED: f32 = 0.5;
    const ENEMY_CHASE_RADIUS: f32 = 6.0;
    const ENEMY_STOP_DIST: f32 = 0.15; 
    let chase_speed = ENEMY_CHASE_SPEED * block_size as f32;
    let activation_dist_sq = (ENEMY_CHASE_RADIUS * block_size as f32).powi(2);
    let stop_dist_sq = (ENEMY_STOP_DIST * block_size as f32).powi(2);
    for f in &mut enemies { 
        if f.id != 'f' { continue; }
        let dx = player.pos.x - f.pos.x;
        let dy = player.pos.y - f.pos.y;
        let dist_sq = dx*dx + dy*dy;
        if dist_sq > activation_dist_sq || dist_sq <= stop_dist_sq { continue; }
        let dist = dist_sq.sqrt();
        if dist < 1.0 { continue; }
        let nx = dx / dist;
        let ny = dy / dist;
        let step = chase_speed * dt;
        let enemy_radius = 14.0;
        let try_x = f.pos.x + nx * step;
        if is_walkable_with_radius(try_x, f.pos.y, maze_ref, block_size, enemy_radius) { f.pos.x = try_x; }
        let try_y = f.pos.y + ny * step;
        if is_walkable_with_radius(f.pos.x, try_y, maze_ref, block_size, enemy_radius) { f.pos.y = try_y; }
    }

    // if the enemy finds the player, game over
    if game_state == GameState::Playing {
        let p_cx = (player.pos.x / block_size as f32) as isize;
        let p_cy = (player.pos.y / block_size as f32) as isize;
        for e in &enemies {
            if e.id == 'f' {
                let ecx = (e.pos.x / block_size as f32) as isize;
                let ecy = (e.pos.y / block_size as f32) as isize;
                if ecx == p_cx && ecy == p_cy {
                    game_state = GameState::GameOver;
                    break;
                }
            }
        }
    }

    //checks if the player has picked up the key

    if !has_key {
        let pickup_radius = 0.55 * block_size as f32; 
        let r2 = pickup_radius * pickup_radius;
        let mut picked = false;
        enemies.retain(|e| {
            if e.id == 'k' {
                let dx = e.pos.x - player.pos.x;
                let dy = e.pos.y - player.pos.y;
                if dx * dx + dy * dy < r2 { picked = true; return false; }
            }
            true
        });
        if picked { has_key = true; }
    }

    // victory if the player has the key and is near the iglo
    if has_key && game_state == GameState::Playing {
        let player_x = player.pos.x;
        let player_y = player.pos.y;
        let player_cell_x = (player_x / block_size as f32) as isize;
        let player_cell_y = (player_y / block_size as f32) as isize;
        let search_radius_cells: isize = 2;
        let proximity_dist = 0.8 * block_size as f32;
        let proximity_sq = proximity_dist * proximity_dist;
        'outer: for cy in (player_cell_y - search_radius_cells)..=(player_cell_y + search_radius_cells) {
            if cy < 0 || cy as usize >= maze_ref.len() { continue; }
            for cx in (player_cell_x - search_radius_cells)..=(player_cell_x + search_radius_cells) {
                if cx < 0 || cx as usize >= maze_ref[0].len() { continue; }
                if maze_ref[cy as usize][cx as usize] == 'g' {
                    let center_x = (cx as f32 + 0.5) * block_size as f32;
                    let center_y = (cy as f32 + 0.5) * block_size as f32;
                    let dx = center_x - player_x;
                    let dy = center_y - player_y;
                    if dx*dx + dy*dy <= proximity_sq {
                        game_state = GameState::Win;
                        break 'outer;
                    }
                }
            }
        }
    }

    //Sound of taylor swift if the player is near the puffle
    let tay_proximity: f32 = 200.0;
    let mut any_p_in_range = false;
    for enemy in &enemies {
        if enemy.id == 'p' {
            let dx = enemy.pos.x - player.pos.x;
            let dy = enemy.pos.y - player.pos.y;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq < tay_proximity * tay_proximity { any_p_in_range = true; break; }
        }
    }
    if any_p_in_range { if !taylor.is_playing() { taylor.play(); } } else { if taylor.is_playing() { taylor.stop(); } }

    framebuffer.clear();

    let proj_plane = (framebuffer.width as f32) / (2.0 * (player.fov * 0.5).tan());

    render_world(&mut framebuffer, maze_ref, block_size, &player, &tex_manager, &mut depth_buffer);

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
    let mini_h = maze_ref.len() as u32 * cell_px;
    let margin = 8;
    let ox = margin;
    let oy = framebuffer.height - mini_h - margin;
    render_minimap(&mut framebuffer, maze_ref, block_size, &player, ox, oy, cell_px);

    //show key icon on HUD if the player has the key
    if has_key { draw_key_hud_icon(&mut framebuffer, &tex_manager); }

    framebuffer.swap_buffers(&mut window, &raylib_thread, true);
    thread::sleep(Duration::from_millis(16));
  }

  
  fn draw_key_hud_icon(framebuffer: &mut Framebuffer, tex: &TextureManager) {
      let (tw, th) = tex.get_size('k');
      if tw == 0 || th == 0 { return; }
      let dest_h: u32 = 56; // tamaño en pantalla
      let dest_w: u32 = (dest_h as f32 * (tw as f32 / th as f32)) as u32;
      let margin: u32 = 10;
      let x0 = framebuffer.width.saturating_sub(dest_w + margin);
      let y0 = margin;
      for dy in 0..dest_h {
          let ty = (dy as f32 / dest_h as f32 * th as f32) as u32;
          for dx in 0..dest_w {
              let tx = (dx as f32 / dest_w as f32 * tw as f32) as u32;
              let c = tex.sample('k', tx, ty);
              // Ignorar color magenta dummy como transparencia
              if (c.r, c.g, c.b) == (255, 0, 255) { continue; }
              framebuffer.set_pixel_color(x0 + dx, y0 + dy, c);
          }
      }
  }

  // Check if a position is walkable in the maze
  fn is_walkable_world(x: f32, y: f32, maze: &Maze, block_size: usize) -> bool {
      let cx = (x / block_size as f32) as isize;
      let cy = (y / block_size as f32) as isize;
      if cy < 0 || cx < 0 { return false; }
      if cy as usize >= maze.len() || cx as usize >= maze[0].len() { return false; }
      maze[cy as usize][cx as usize] == ' '
  }

  fn is_walkable_with_radius(x: f32, y: f32, maze: &Maze, block_size: usize, radius: f32) -> bool {
      let points = [
          (x, y),
          (x + radius, y),
          (x - radius, y),
          (x, y + radius),
          (x, y - radius),
      ];
      for (px, py) in points {
          if px < 0.0 || py < 0.0 { return false; }
          let gx = (px as usize) / block_size;
          let gy = (py as usize) / block_size;
          if gy >= maze.len() || gx >= maze[0].len() { return false; }
          if maze[gy][gx] != ' ' { return false; }
      }
      true
  }
}
