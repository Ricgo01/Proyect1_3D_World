use raylib::prelude::*;
use std::f32::consts::PI;

use crate::player::Player;
use crate::framebuffer::Framebuffer;
use crate::textures::TextureManager;

pub const TRANSPARENT_COLOR: Color = Color::new(152, 0, 136, 255);
// Factor base para reducir la altura original (comparada con un bloque de muro)
pub const ENEMY_BASE_SCALE: f32 = 0.5;

pub struct Enemy {
    pub pos: Vector2,
    pub id: char,
    pub scale: f32, // factor adicional encima de ENEMY_BASE_SCALE
}

impl Enemy {
    pub fn new(x: f32, y: f32, id: char) -> Self {
        Self {
            pos: Vector2::new(x, y),
            id,
            scale: 1.0,
        }
    }
    pub fn with_scale(x: f32, y: f32, id: char, scale: f32) -> Self {
        Self { pos: Vector2::new(x, y), id, scale }
    }
}

pub fn draw_sprites(
    framebuffer: &mut Framebuffer,
    player: &Player,
    enemies: &mut [Enemy],
    tex: &TextureManager,
    depth_buffer: &[f32],
    proj_plane: f32,
    block_size: usize,
) {
    if enemies.is_empty() {
        return;
    }

    // --- CAMBIO CORREGIDO ---
    // Ordenar por distancia (más lejanos primero para painter's algorithm)
    // Se revierte al cálculo manual de la distancia al cuadrado, ya que `length_sq()` no existe.
    enemies.sort_by(|a, b| {
        let da = (a.pos.x - player.pos.x).powi(2) + (a.pos.y - player.pos.y).powi(2);
        let db = (b.pos.x - player.pos.x).powi(2) + (b.pos.y - player.pos.y).powi(2);
        db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
    });

    let screen_w = framebuffer.width;
    let screen_h = framebuffer.height;
    let half_screen_w = screen_w as f32 * 0.5;
    let half_screen_h = screen_h as f32 * 0.5;
    let half_fov = player.fov * 0.5;

    for enemy in enemies.iter() {
        // 1. Calcular ángulo y distancia al sprite
        let dx = enemy.pos.x - player.pos.x;
        let dy = enemy.pos.y - player.pos.y;
        let dist = (dx * dx + dy * dy).sqrt();

        let sprite_angle = dy.atan2(dx);

        // 2. Normalizar la diferencia angular al rango [-PI, PI]
        let mut angle_diff = sprite_angle - player.a;
        if angle_diff > PI { angle_diff -= 2.0 * PI; }
        if angle_diff < -PI { angle_diff += 2.0 * PI; }

        // 3. Verificar si el sprite está dentro del FOV (con un pequeño margen para evitar popping)
        if angle_diff.abs() > half_fov + 0.2 {
            continue;
        }

        // 4. Corregir la distancia para evitar "ojo de pez"
        let dist_corrected = dist * angle_diff.cos();
        if dist_corrected <= 0.5 { continue; } // Aumentar plano de corte cercano

        // 5. Calcular la posición y tamaño del sprite en la pantalla
        let sprite_real_height = block_size as f32 * ENEMY_BASE_SCALE * enemy.scale;
        let sprite_height = (sprite_real_height * proj_plane) / dist_corrected;

        let (tw, th) = tex.get_size(enemy.id);
        if tw == 0 || th == 0 { continue; }
        let aspect_ratio = tw as f32 / th as f32;
        let sprite_width = sprite_height * aspect_ratio;

        let sprite_screen_x = half_screen_w + (angle_diff / half_fov) * half_screen_w;

        // 6. Calcular los límites de dibujo (start/end para X/Y)
        let v_move_screen = sprite_height * 0.5; 
        
        let draw_start_y = (half_screen_h - sprite_height * 0.5 + v_move_screen).max(0.0) as i32;
        let draw_end_y = (half_screen_h + sprite_height * 0.5 + v_move_screen).min(screen_h as f32) as i32;

        let draw_start_x = (sprite_screen_x - sprite_width * 0.5).max(0.0) as i32;
        let draw_end_x = (sprite_screen_x + sprite_width * 0.5).min(screen_w as f32) as i32;

        // --- NUEVO: CULLING POR OCULSION TOTAL ANTES DE RECORRER STRIPES ---
        // Si todas las columnas que ocuparía el sprite tienen una pared más cercana en depth_buffer,
        // podemos descartar el sprite completo.
        if draw_end_x > draw_start_x {
            let sx0 = draw_start_x.max(0) as usize;
            let mut sx1 = draw_end_x.max(0) as usize;
            if sx1 >= screen_w as usize { sx1 = screen_w as usize - 1; }
            if sx0 < sx1 && sx1 < depth_buffer.len() {
                // Muestreamos cada 2 columnas para reducir costo; buscamos la mínima profundidad de pared.
                let mut min_depth = f32::INFINITY;
                let mut i = sx0;
                while i <= sx1 {
                    let d = depth_buffer[i];
                    if d < min_depth { min_depth = d; }
                    i += 2; // salto
                }
                // También incluimos la última si no cayó exacto en el salto
                let last_d = depth_buffer[sx1];
                if last_d < min_depth { min_depth = last_d; }
                // Compare usando un pequeño margen (0.98) para evitar errores por diferencias de proyección.
                if dist_corrected > min_depth * 0.98 {
                    continue; // totalmente detrás de paredes visibles
                }
            }
        }

        // 7. Dibujar las columnas verticales del sprite (stripes)
        for stripe in draw_start_x..draw_end_x {
            let stripe_idx = stripe as usize;
            if stripe_idx >= screen_w as usize { continue; }

            // Oclusión con el Z-buffer de las paredes (por-columna)
            if dist_corrected > depth_buffer[stripe_idx] {
                continue;
            }

            // Coordenada X de la textura
            if sprite_width < 1.0 { continue; }
            let tex_x = (((stripe - draw_start_x) as f32 / sprite_width) * tw as f32) as u32;
            
            for y in draw_start_y..draw_end_y {
                // Coordenada Y de la textura
                if sprite_height < 1.0 { continue; }
                let tex_y = (((y - draw_start_y) as f32 / sprite_height) * th as f32) as u32;

                let color = tex.sample(enemy.id, tex_x, tex_y);

                if color == TRANSPARENT_COLOR || color.a == 0 { continue; }

                // Sombreado por distancia
                let shade = (1.0 / (1.0 + dist_corrected * 0.1)).clamp(0.3, 1.0);
                let final_color = Color {
                    r: (color.r as f32 * shade) as u8,
                    g: (color.g as f32 * shade) as u8,
                    b: (color.b as f32 * shade) as u8,
                    a: 255,
                };

                framebuffer.set_pixel_color(stripe as u32, y as u32, final_color);
            }
        }
    }
}