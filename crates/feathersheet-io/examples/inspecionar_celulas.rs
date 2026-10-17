//! Mostra o conteúdo e o valor de um canto de cada aba de um `.fshz`:
//! `inspecionar_celulas ARQUIVO.fshz [linhas] [colunas]`.
fn main() {
    let caminho = std::path::PathBuf::from(std::env::args().nth(1).expect("arquivo"));
    let linhas: u32 = std::env::args().nth(2).and_then(|n| n.parse().ok()).unwrap_or(4);
    let colunas: u32 = std::env::args().nth(3).and_then(|n| n.parse().ok()).unwrap_or(14);
    let lida = feathersheet_io::fshz_read::ler_fshz_arquivo(&caminho).expect("ler FSHZ");
    let w = feathersheet_io::para_workbook::para_workbook(&lida);
    for i in 0..w.len() {
        let s = &w.aba(i).unwrap().sheet;
        println!("== {} · área usada {:?} · {} células", s.name, (s.max_row, s.max_col), s.len());
        for r in 0..linhas {
            let linha: Vec<String> = (0..colunas).map(|c| {
                let raw = s.get_raw(r, c);
                if raw.starts_with('=') { format!("{raw} → {:?}", w.valor(i, r, c)) } else { raw }
            }).collect();
            println!("  {}: {}", r + 1, linha.join(" | "));
        }
    }
}
