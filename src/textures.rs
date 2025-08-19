use raylib::prelude::*;
use std::collections::HashMap;

pub struct CpuTexture {
    pub w: u32,
    pub h: u32,
    pub pixels: Vec<u8>, // RGBA8
}

/// Maneja texturas en CPU (formato normalizado RGBA8) para muestreo seguro.
pub struct TextureManager {
    tex: HashMap<char, CpuTexture>,   // paredes
    sky: Option<CpuTexture>,          // textura de cielo opcional
    ground: Option<CpuTexture>,       // textura de suelo opcional
}

impl TextureManager {
    pub fn new() -> Self { Self { tex: HashMap::new(), sky: None, ground: None } }

    pub fn load_defaults(&mut self) {
        let files = [
            ('+', "assets/secret.png"),
            ('-', "assets/dance.png"),
            ('|', "assets/mall.png"),
            ('e', "assets/enemy.png"),
            ('f', "assets/enemy2.png"),
            ('k', "assets/key.png"),
            ('p', "assets/puffle.png")
        ];
        for (ch, path) in files { self.load_one(ch, path); }
        // Cargar cielo y suelo
        self.load_sky("assets/sky.png");
        self.load_ground("assets/ice1.png");
    }

    fn load_sky(&mut self, path: &str) {
        if let Ok(img) = Image::load_image(path) {
            if let Some(ct) = Self::to_cpu_texture(img) { self.sky = Some(ct); }
        } else { eprintln!("No se pudo cargar cielo {path}, se usará gradiente"); }
    }

    fn load_ground(&mut self, path: &str) {
        if let Ok(img) = Image::load_image(path) {
            if let Some(ct) = Self::to_cpu_texture(img) { self.ground = Some(ct); }
        } else { eprintln!("No se pudo cargar suelo {path}, se usará color"); }
    }

    fn load_one(&mut self, ch: char, path: &str) {
        match Image::load_image(path) {
            Ok(img) => {
                if let Some(ct) = Self::to_cpu_texture(img) { self.tex.insert(ch, ct); } else { self.insert_dummy(ch); }
            }
            Err(e) => {
                eprintln!("No se pudo cargar {path}: {e} (dummy)");
                self.insert_dummy(ch);
            }
        }
    }

    fn to_cpu_texture(img: Image) -> Option<CpuTexture> {
        // Rechazar formatos comprimidos para simplicidad
        use PixelFormat::*;
        let fmt = img.format();
        let (w, h) = (img.width.max(1) as u32, img.height.max(1) as u32);
        let mut rgba: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
        unsafe {
            let ptr = img.data as *const u8;
            if ptr.is_null() { return None; }
            let src_slice = std::slice::from_raw_parts(ptr, (w * h * Self::bpp(fmt) as u32) as usize);
            match fmt {
                PIXELFORMAT_UNCOMPRESSED_R8G8B8A8 => { rgba.extend_from_slice(src_slice); }
                PIXELFORMAT_UNCOMPRESSED_R8G8B8 => {
                    for px in src_slice.chunks_exact(3) { rgba.extend_from_slice(&[px[0], px[1], px[2], 255]); }
                }
                PIXELFORMAT_UNCOMPRESSED_GRAYSCALE => { for g in src_slice { rgba.extend_from_slice(&[*g, *g, *g, 255]); } }
                PIXELFORMAT_UNCOMPRESSED_GRAY_ALPHA => { for ga in src_slice.chunks_exact(2) { rgba.extend_from_slice(&[ga[0], ga[0], ga[0], ga[1]]); } }
                _ => { eprintln!("Formato no soportado ({fmt:?}) se usa dummy"); return None; }
            }
        }
        if rgba.len() != (w * h * 4) as usize { return None; }
        Some(CpuTexture { w, h, pixels: rgba })
    }

    #[inline]
    fn bpp(fmt: PixelFormat) -> usize {
        use PixelFormat::*;
        match fmt {
            PIXELFORMAT_UNCOMPRESSED_R8G8B8A8 => 4,
            PIXELFORMAT_UNCOMPRESSED_R8G8B8 => 3,
            PIXELFORMAT_UNCOMPRESSED_GRAY_ALPHA => 2,
            PIXELFORMAT_UNCOMPRESSED_GRAYSCALE => 1,
            _ => 4,
        }
    }

    fn insert_dummy(&mut self, ch: char) {
        let mut pixels = Vec::with_capacity(64*64*4);
        for y in 0..64 { for x in 0..64 { let c = if (x/8 + y/8) % 2 == 0 {(255,0,255)} else {(0,0,0)}; pixels.extend_from_slice(&[c.0,c.1,c.2,255]); } }
        self.tex.insert(ch, CpuTexture { w:64, h:64, pixels });
    }

    pub fn get_size(&self, ch: char) -> (u32,u32) { self.tex.get(&ch).map(|t|(t.w,t.h)).unwrap_or((1,1)) }

    pub fn sample(&self, ch: char, tx: u32, ty: u32) -> Color {
        if let Some(t) = self.tex.get(&ch) {
            let x = tx.min(t.w-1); let y = ty.min(t.h-1); let idx = ((y*t.w + x)*4) as usize; let p=&t.pixels; if idx+3 < p.len() { return Color::new(p[idx],p[idx+1],p[idx+2],p[idx+3]); }
        }
        Color::MAGENTA
    }

    pub fn sample_sky(&self, u: f32, v: f32) -> Color {
        if let Some(s) = &self.sky {
            let uu = (u.rem_euclid(1.0) * s.w as f32) as u32;
            let vv = (v.clamp(0.0,1.0) * s.h as f32) as u32;
            let idx = ((vv * s.w + uu) * 4) as usize;
            if idx + 3 < s.pixels.len() { return Color::new(s.pixels[idx], s.pixels[idx+1], s.pixels[idx+2], s.pixels[idx+3]); }
        }
        let t = v.clamp(0.0,1.0);
        Color::new((30.0 + 50.0*(1.0-t)) as u8, (50.0 + 80.0*(1.0-t)) as u8, (120.0 + 80.0*t) as u8, 255)
    }

    pub fn sample_ground(&self, u: f32, v: f32) -> Color {
        if let Some(g) = &self.ground {
            let uu = (u.rem_euclid(1.0) * g.w as f32) as u32;
            let vv = (v.rem_euclid(1.0) * g.h as f32) as u32;
            let idx = ((vv * g.w + uu) * 4) as usize;
            if idx + 3 < g.pixels.len() { return Color::new(g.pixels[idx], g.pixels[idx+1], g.pixels[idx+2], g.pixels[idx+3]); }
        }
        Color::new(40, 40, 60, 255)
    }
}