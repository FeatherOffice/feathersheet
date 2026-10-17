//! Abertura de CSV em duas partes: só dividir o texto e a importação inteira.
//! `medir_csv ARQUIVO.csv [separador]` (sem separador = o do inglês, `,`).
use std::time::Instant;

fn main() {
    let caminho = std::path::PathBuf::from(std::env::args().nth(1).expect("arquivo"));
    let separador = std::env::args().nth(2).and_then(|s| s.chars().next()).or(Some(','));

    let t = Instant::now();
    let arquivo = std::fs::File::open(&caminho).expect("abrir");
    let leitor = std::io::BufReader::with_capacity(64 * 1024, arquivo);
    let (mut linhas, mut campos) = (0u64, 0u64);
    feather_io::csv_lotes::dividir_csv_leitor_preferindo(leitor, separador, 512, |lote: &mut Vec<Vec<String>>| {
        for l in lote.drain(..) {
            linhas += 1;
            campos += l.len() as u64;
        }
    }).expect("dividir");
    println!("[só dividir] {linhas} linhas, {campos} campos em {:.2} s", t.elapsed().as_secs_f64());

    let t = Instant::now();
    let w = feathersheet_io::csv_workbook::importar_csv_tsv_arquivo(&caminho, false, separador).expect("importar");
    println!("[importar] {:.2} s · {} linhas", t.elapsed().as_secs_f64(), w.aba(0).unwrap().sheet.len());
    for linha in feathersheet_core::diario::recolher() {
        println!("  {linha}");
    }
}
