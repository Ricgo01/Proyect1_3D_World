use raylib::prelude::*;
use std::collections::HashMap;

pub struct CpuTexture {
    pub w: u32,
    pub h: u32,
    pub pixels: Vec<u8>, // RGBA8
}

/// Maneja texturas en CPU (formato normalizado RGBA8) para muestreo seguro.
pub struct TextureManager {
    tex: HashMap<char, CpuTexture>,
}

impl TextureManager {
    pub fn new() -> Self { Self { tex: HashMap::new() } }

    pub fn load_defaults(&mut self) {
        let files = [
            ('+', "assets/secret.png"),
            ('-', "assets/dance.png"),
            ('|', "assets/mall.png"),
            ('g', "assets/wall4.png"),
            ('#', "assets/wall5.png"),
        ];
        for (ch, path) in files { self.load_one(ch, path); }
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
            // bytes totales estimados (raylib guarda tamaño acorde formato)
            let src_slice = std::slice::from_raw_parts(ptr, (w * h * Self::bpp(fmt) as u32) as usize);
            match fmt {
                PIXELFORMAT_UNCOMPRESSED_R8G8B8A8 => {
                    rgba.extend_from_slice(src_slice);
                }
                PIXELFORMAT_UNCOMPRESSED_R8G8B8 => {
                    for px in src_slice.chunks_exact(3) {
                        rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
                    }
                }
                PIXELFORMAT_UNCOMPRESSED_GRAYSCALE => {
                    for g in src_slice { rgba.extend_from_slice(&[*g, *g, *g, 255]); }
                }
                PIXELFORMAT_UNCOMPRESSED_GRAY_ALPHA => {
                    for ga in src_slice.chunks_exact(2) { rgba.extend_from_slice(&[ga[0], ga[0], ga[0], ga[1]]); }
                }
                _ => {
                    eprintln!("Formato no soportado ({fmt:?}) se usa dummy");
                    return None;
                }
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
}