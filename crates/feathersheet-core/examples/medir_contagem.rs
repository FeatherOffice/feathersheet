//! Quanto o rodapé (Contagem/Soma/Média) custa por seleção:
//! `cargo run --release --example medir_contagem`.
use feathersheet_core::{celulas::refs::Rect, planilha::sheet::Sheet};
use std::time::Instant;
fn main() {
    let mut s = Sheet::new("x");
    for r in 0..200_000u32 {
        s.set_cell(r, 0, &(r % 97).to_string());
        if r < 50_000 { s.set_cell(r, 1, &format!("=A{}*2", r + 1)); }
    }
    for (nome, area) in [("A1:A200000 (números)", Rect::new(0, 0, 199_999, 0)),
        ("B1:B50000 (fórmulas)", Rect::new(0, 1, 49_999, 1)),
        ("A1:A20000", Rect::new(0, 0, 19_999, 0))] {
        let _ = s.contar_selecao(&area, None);
        let t = Instant::now();
        let c = s.contar_selecao(&area, None);
        println!("{nome}: {:.1} ms · contagem {}", t.elapsed().as_secs_f64() * 1000.0, c.count);
    }
}
