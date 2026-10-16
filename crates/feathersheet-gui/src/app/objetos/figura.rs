//! Figuras da camada de objetos (`tipo: "imagem"`, vindas do `.xlsx`):
//! decodifica o `data:` URI uma vez, aplica recorte e duotom do Office e
//! guarda o `Handle` num cache por conteúdo (o desenho pede a cada quadro).
use base64::{engine::general_purpose::STANDARD, Engine as _};
use feathersheet_io::objetos_aba::ImagemObjeto;
use iced::widget::image::Handle;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Teto de figuras decodificadas guardadas.
const MAX_CACHE: usize = 128;

thread_local! {
    static CACHE: RefCell<HashMap<u64, Option<Handle>>> = RefCell::new(HashMap::new());
}

fn chave(im: &ImagemObjeto) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    im.src.hash(&mut h);
    im.duotom.hash(&mut h);
    im.recorte.map(|r| r.map(f32::to_bits)).hash(&mut h);
    h.finish()
}

/// O `Handle` da figura (com a chave do cache), ou `None` se não decodifica.
pub(super) fn handle(im: &ImagemObjeto) -> Option<(u64, Handle)> {
    let k = chave(im);
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if let Some(h) = c.get(&k) {
            return h.clone().map(|h| (k, h));
        }
        if c.len() >= MAX_CACHE {
            c.clear();
        }
        let h = decodificar(im);
        c.insert(k, h.clone());
        h.map(|h| (k, h))
    })
}

fn rgb(hex: &str) -> [f32; 3] {
    let [r, g, b, _] = feather_ui::seletor_cor::rgba_de(hex);
    [f32::from(r), f32::from(g), f32::from(b)]
}

/// Pixels RGBA da figura com recorte e duotom aplicados.
pub(super) fn decodificar(im: &ImagemObjeto) -> Option<Handle> {
    let dados = im.src.split_once(";base64,")?.1;
    let bytes = STANDARD.decode(dados.trim()).ok()?;
    let mut img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    if let Some([l, t, r, b]) = im.recorte {
        let (w, h) = (img.width() as f32, img.height() as f32);
        let x0 = (l.max(0.0) * w) as u32;
        let y0 = (t.max(0.0) * h) as u32;
        let x1 = ((1.0 - r.max(0.0)) * w) as u32;
        let y1 = ((1.0 - b.max(0.0)) * h) as u32;
        if x1 > x0 && y1 > y0 {
            img = image::imageops::crop_imm(&img, x0, y0, x1 - x0, y1 - y0).to_image();
        }
    }
    if let Some([escura, clara]) = &im.duotom {
        let (e, c) = (rgb(escura), rgb(clara));
        for p in img.pixels_mut() {
            let lum = (0.299 * f32::from(p[0]) + 0.587 * f32::from(p[1]) + 0.114 * f32::from(p[2])) / 255.0;
            for i in 0..3 {
                p[i] = (e[i] + (c[i] - e[i]) * lum).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    Some(Handle::from_rgba(img.width(), img.height(), img.into_raw()))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn duotom_pinta_o_escuro_e_mantem_o_alfa() {
        let mut png = Vec::new();
        let img = image::RgbaImage::from_pixel(2, 1, image::Rgba([0, 0, 0, 128]));
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        let im = ImagemObjeto {
            src: format!("data:image/png;base64,{}", STANDARD.encode(&png)),
            duotom: Some(["#b38600".into(), "#ffffff".into()]),
            recorte: Some([0.5, 0.0, 0.0, 0.0]),
        };
        let Some(Handle::Rgba { width, height, pixels, .. }) = decodificar(&im) else { panic!("sem figura") };
        assert_eq!((width, height), (1, 1));
        assert_eq!(&pixels[..], &[0xb3, 0x86, 0x00, 128]);
        assert!(handle(&im).is_some() && handle(&im).unwrap().0 == chave(&im));
    }
}
