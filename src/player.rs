use raylib::prelude::*;
use std::f32::consts::PI;
use crate::maze::Maze;

pub struct Player {
    pub pos: Vector2,
    pub a: f32,
    pub fov: f32,
}

fn colision (maze: &Maze, block_size: usize, x: f32, y: f32, radius: f32) -> bool {
    //this list contains the points that are used to check if the player is colliding with a wall
    let hitbox = [
        (x, y),
        (x + radius, y),
        (x - radius, y),
        (x, y + radius),
        (x, y - radius),
    ];
    //check if the points are inside the maze and if they are not colliding with a wall
    //if any of the points are colliding with a wall, return false
    for (i, j) in hitbox{
        if i < 0.0 || j < 0.0 {return false}
        let gx = (i as usize) / block_size;
        let gy = (j as usize) / block_size;
        if gy >= maze.len() || gx >= maze[0].len() { return false; }
        if maze[gy][gx] != ' ' { return false; }
    } 
    true
}

// function that procces the player events this is called in th emain render loop
pub fn process_events(player: &mut Player, rl: &RaylibHandle, dt: f32, maze: &Maze, block_size: usize) {
    //Velocities of forward, lateral, rotation and the mouse movement for the player
    //Change these values to increse or deacrese the movement speed
    const MOVE_SPEED: f32 = 36.0;
    const LATERAL_SPEED: f32 = 36.0;
    const ROTATION_SPEED: f32 = PI / 30.0;
    const MOUSE_MOVE_SPEED: f32 = 0.0025;
    const PLAYER_RADIUS: f32 = 12.0;

    //mouse delta to rotate the player
    let md = rl.get_mouse_delta();
    player.a += md.x * MOUSE_MOVE_SPEED;

    // Keep the angle within the range of -PI to PI
    if player.a > PI { player.a -= 2.0 * PI; }
    if player.a < -PI { player.a += 2.0 * PI; }

    // mutables variables to calculate the forward and right vectors
    let forward = Vector2::new(player.a.cos(), player.a.sin());
    let right = Vector2::new(-forward.y, forward.x);

    // Movimiento inmediato: intentamos cada dirección y sólo aplicamos si no hay colisión
    let mut new_x = player.pos.x;
    let mut new_y = player.pos.y;

    // Avanzar
    if rl.is_key_down(KeyboardKey::KEY_W) || rl.is_key_down(KeyboardKey::KEY_UP) {
        let tx = new_x + forward.x * MOVE_SPEED * dt;
        let ty = new_y + forward.y * MOVE_SPEED * dt;
        if colision(maze, block_size, tx, ty, PLAYER_RADIUS) {
            new_x = tx;
            new_y = ty;
        }
    }
    // Retroceder
    if rl.is_key_down(KeyboardKey::KEY_S) || rl.is_key_down(KeyboardKey::KEY_DOWN) {
        let tx = new_x - forward.x * MOVE_SPEED * dt;
        let ty = new_y - forward.y * MOVE_SPEED * dt;
        if colision(maze, block_size, tx, ty, PLAYER_RADIUS) {
            new_x = tx;
            new_y = ty;
        }
    }
    // Strafe derecha
    if rl.is_key_down(KeyboardKey::KEY_D) || rl.is_key_down(KeyboardKey::KEY_RIGHT) {
        let tx = new_x + right.x * LATERAL_SPEED * dt;
        let ty = new_y + right.y * LATERAL_SPEED * dt;
        if colision(maze, block_size, tx, ty, PLAYER_RADIUS) {
            new_x = tx;
            new_y = ty;
        }
    }
    // Strafe izquierda
    if rl.is_key_down(KeyboardKey::KEY_A) || rl.is_key_down(KeyboardKey::KEY_LEFT) {
        let tx = new_x - right.x * LATERAL_SPEED * dt;
        let ty = new_y - right.y * LATERAL_SPEED * dt;
        if colision(maze, block_size, tx, ty, PLAYER_RADIUS) {
            new_x = tx;
            new_y = ty;
        }
    }

    // Asignar al final
    player.pos.x = new_x;
    player.pos.y = new_y;
}