//! Exportar `.xlsx` e CSV da planilha do usuário, pelo Rust:
//! `cargo run --release --example medir_exportar -- arquivo.fshz`.
//! Mede separado o que segura a ponte (escrever os XML) e o que não
//! segura (comprimir).
use serde_json::Value;
use std::time::Instant;

fn main() {
    let caminho = std::env::args().nth(1).expect("o caminho do .fshz");
    let bytes = std::fs::read(&caminho).expect("ler");
    let mut p = feathersheet_ponte::Ponte::default();
    let t = Instant::now();
    let r: Value = serde_json::from_slice(&p.abrir_fshz(&bytes).unwrap()).unwrap();
    drop(bytes);
    let aba = &r["estado"]["abas"][0];
    let id = aba["id"].as_u64().unwrap() as u32;
    eprintln!("aberto em {} ms: {} linhas × {} colunas", t.elapsed().as_millis(), aba["maxRow"], aba["maxCol"]);

    let t = Instant::now();
    let partes = p.partes_xlsx();
    let cru: usize = partes.iter().map(|(_, b)| b.len()).sum();
    eprintln!("xlsx, escrever os XML (com a ponte): {} ms, {} MB de XML", t.elapsed().as_millis(), cru >> 20);
    let t = Instant::now();
    let zip = feathersheet_io::zip_write::zipar(partes).unwrap();
    eprintln!("xlsx, comprimir (sem a ponte): {} ms, {} MB", t.elapsed().as_millis(), zip.len() >> 20);
    drop(zip);

    let t = Instant::now();
    let csv = p.exportar_delimitado(id, ',').unwrap();
    eprintln!("csv da primeira aba: {} ms, {} MB", t.elapsed().as_millis(), csv.len() >> 20);
}
