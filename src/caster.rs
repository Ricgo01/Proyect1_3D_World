use raylib::color::Color;
use crate::framebuffer::Framebuffer;
use crate::maze::Maze;
use crate::player::Player;

pub struct Intersect {
  pub distance: f32,    // distancia perpendicular (en unidades de mundo / pixeles)
  pub impact: char,     // caracter de pared
  pub hit_x: f32,       // coord mundo del impacto (centro aproximado celda)
  pub hit_y: f32,
  pub side: i32,        // 0 = cara vertical golpeada, 1 = cara horizontal
  pub wall_x: f32,      // fracción [0,1) horizontal dentro de la pared (coord textura)
}

pub fn cast_ray(
  framebuffer: &mut Framebuffer,
  maze: &Maze,
  player: &Player,
  a: f32,
  block_size: usize,
  draw_line: bool,
) -> Intersect {
  if draw_line { return cast_ray_debug(framebuffer, maze, player, a, block_size); }

  let maze_h = maze.len();
  if maze_h == 0 { return empty(); }
  let maze_w = maze[0].len();

  // Posición jugador en coordenadas de grid
  let pos_x = player.pos.x / block_size as f32;
  let pos_y = player.pos.y / block_size as f32;
  let dir_x = a.cos();
  let dir_y = a.sin();

  // Distancias delta para cruzar una celda completa en X/Y
  let delta_x = if dir_x == 0.0 { f32::INFINITY } else { (1.0 / dir_x).abs() };
  let delta_y = if dir_y == 0.0 { f32::INFINITY } else { (1.0 / dir_y).abs() };

  // Celda actual
  let mut map_x = pos_x.floor() as isize;
  let mut map_y = pos_y.floor() as isize;

  // Distancia inicial hasta primer lado en X e Y + pasos
  let (step_x, mut side_dist_x) = if dir_x < 0.0 {
    (-1, (pos_x - map_x as f32) * delta_x)
  } else { (1, ((map_x as f32 + 1.0) - pos_x) * delta_x) };
  let (step_y, mut side_dist_y) = if dir_y < 0.0 {
    (-1, (pos_y - map_y as f32) * delta_y)
  } else { (1, ((map_y as f32 + 1.0) - pos_y) * delta_y) };

  let mut side: i32; // 0 vertical, 1 horizontal

  for _ in 0..10_000 { // límite seguridad
    if side_dist_x < side_dist_y {
      side_dist_x += delta_x;
      map_x += step_x;
      side = 0;
    } else {
      side_dist_y += delta_y;
      map_y += step_y;
      side = 1;
    }

    if map_x < 0 || map_y < 0 || map_y as usize >= maze_h || map_x as usize >= maze_w {
      return empty();
    }

    let cell = maze[map_y as usize][map_x as usize];
    if cell != ' ' {
      let perp_dist = if side == 0 {
        (map_x as f32 - pos_x + (1 - step_x) as f32 / 2.0) / dir_x
      } else {
        (map_y as f32 - pos_y + (1 - step_y) as f32 / 2.0) / dir_y
      };

      // wall_x
      let mut wall_x = if side == 0 { pos_y + perp_dist * dir_y } else { pos_x + perp_dist * dir_x };
      wall_x -= wall_x.floor();

      let hit_x = (map_x as f32 + 0.5) * block_size as f32;
      let hit_y = (map_y as f32 + 0.5) * block_size as f32;

      return Intersect { distance: perp_dist * block_size as f32, impact: cell, hit_x, hit_y, side, wall_x };
    }
  }

  empty()
}

fn empty() -> Intersect {
  Intersect { distance: 0.0, impact: ' ', hit_x: 0.0, hit_y: 0.0, side: 0, wall_x: 0.0 }
}

// Ray debug (ray marching simple) para modo 2D.
fn cast_ray_debug(
  framebuffer: &mut Framebuffer,
  maze: &Maze,
  player: &Player,
  a: f32,
  block_size: usize,
) -> Intersect {
  let mut d = 0.0;
  let step = 1.0;
  let max_dist = 8000.0;
  framebuffer.set_current_color(Color::WHITESMOKE);

  loop {
    let xw = player.pos.x + a.cos() * d;
    let yw = player.pos.y + a.sin() * d;
    if xw < 0.0 || yw < 0.0 || d > max_dist { return empty(); }

    let i = (xw as usize) / block_size;
    let j = (yw as usize) / block_size;
    if j >= maze.len() || i >= maze[0].len() { return empty(); }

    let cell = maze[j][i];
    if cell != ' ' {
      return Intersect { distance: d, impact: cell, hit_x: xw, hit_y: yw, side: 0, wall_x: 0.0 };
    }
    if (xw as u32) < framebuffer.width && (yw as u32) < framebuffer.height {
      framebuffer.set_pixel(xw as u32, yw as u32);
    }
    d += step;
  }
}