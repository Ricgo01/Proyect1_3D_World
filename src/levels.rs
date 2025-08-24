use crate::maze::{Maze, load_maze};
use crate::sprites::Enemy;

// class that defines multiples levels on the game

#[derive(Copy, Clone, PartialEq)]
pub enum GameState {
    Start,
    Playing,
    Win,
    GameOver,
}

//this struct defines the level that will be loaded
//recibe the name of the level, the maze path, the player start position and the enemies positions

pub struct LevelDef {
    pub name: &'static str,
    pub maze_path: &'static str,
    pub player_start: (f32, f32, f32),
    pub enemies: &'static [(f32, f32, char)],
}

pub const LEVELS: &[LevelDef] = &[
    LevelDef { name: "Nivel 1", maze_path: "maze.txt", player_start: (13.0, 2.0, 0.0), enemies: &[
        (14.0, 4.0, 'e'), (2.5, 9.5, 'f'), (4.5, 1.5, 'k'), (6.5, 1.5, 'p'), ] },
    LevelDef { name: "Nivel 2", maze_path: "maze2.txt", player_start: (13.0, 2.0, 0.0), enemies: &[
        (11.5, 2.5, 'e'), (2.5, 7.5, 'f'), (2.5, 4.5, 'k'), (4.5, 4.5, 'p')] },
    LevelDef { name: "Nivel 3", maze_path: "maze3.txt", player_start: (15.0, 1.5, 0.0), enemies: &[
        (14.5, 1.5, 'e'), (14.0, 6.5, 'f'), (3.0, 2.5, 'k'), (8.5, 7.5, 'p'), ] },
];

// Load the maze, enemies and player start position for a given level definition

pub fn load_level(def: &LevelDef, block_size: usize) -> (Maze, Vec<Enemy>, (f32, f32, f32)) {
    let maze = load_maze(def.maze_path);
    let enemies = def.enemies.iter()
        .map(|(x, y, id)| Enemy::new(*x * block_size as f32, *y * block_size as f32, *id))
        .collect::<Vec<_>>();
    (maze, enemies, def.player_start)
}
