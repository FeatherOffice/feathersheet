//! Quanto custa a contagem do rodapé (Soma/Média/Contagem) numa coluna e na
//! tabela inteira de um `.parquet` grande.
//!   systemd-run --user --scope -p MemoryMax=16G cargo run --release --example medir_contagem_parquet -- arquivo.parquet
use feathersheet_core::celulas::refs::Rect;
fn main() -> Result<(), String> {
    let caminho = std::env::args().nth(1).ok_or("informe o arquivo")?;
    let wb = feathersheet_io::parquet_read::ler_parquet_arquivo(std::path::Path::new(&caminho))?;
    let s = &wb.aba(0).ok_or("sem aba")?.sheet;
    let u = s.used_rect();
    for (nome, area) in [
        ("coluna F (número)", Rect::new(0, 5, u.r1, 5)),
        ("coluna A (texto)", Rect::new(0, 0, u.r1, 0)),
        ("tabela inteira", Rect::new(0, 0, u.r1, u.c1)),
    ] {
        let t = std::time::Instant::now();
        let c = s.contar_selecao(&area, Some(&wb));
        println!("{nome}: {:.0} ms · contagem {} · soma {}", t.elapsed().as_secs_f64() * 1000.0, c.count, c.sum);
        // Como o rodapé faz hoje: lotes de 4096 linhas.
        let t = std::time::Instant::now();
        let mut r = area.r0;
        while r <= area.r1 {
            let linhas = (500_000 / (area.c1 - area.c0 + 1)).max(1);
            let r1 = (r + linhas - 1).min(area.r1);
            let _ = s.contar_selecao(&Rect::new(r, area.c0, r1, area.c1), Some(&wb));
            r = r1 + 1;
        }
        println!("   em lotes de 500 mil células: {:.0} ms", t.elapsed().as_secs_f64() * 1000.0);
        if nome == "tabela inteira" { break; }
    }
    Ok(())
}
