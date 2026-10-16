//! Ctrl+seta na planilha do usuário, pela ponte:
//! `cargo run --release --example medir_borda -- arquivo.fshz`.
use serde_json::{json, Value};
use std::time::Instant;

fn main() {
    let caminho = std::env::args().nth(1).expect("o caminho do .fshz");
    let bytes = std::fs::read(&caminho).expect("ler");
    let mut p = feathersheet_ponte::Ponte::default();
    let r: Value = serde_json::from_slice(&p.abrir_fshz(&bytes).unwrap()).unwrap();
    let id = r["estado"]["abas"][0]["id"].as_u64().unwrap();
    for (linha, coluna, dr, dc) in [(0, 0, 1, 0), (0, 3, 1, 0), (0, 0, 0, 1), (1_999_999, 0, -1, 0), (5, 0, 1, 0)] {
        let t = Instant::now();
        let args = json!({ "aba": id, "linha": linha, "coluna": coluna, "dr": dr, "dc": dc,
                           "limiteLinha": 10_485_759, "limiteColuna": 16_383 });
        let r = p.chamar("borda", &serde_json::to_vec(&args).unwrap());
        eprintln!("({linha},{coluna}) d=({dr},{dc}): {} ms -> {:?}", t.elapsed().as_millis(), r.map(|b| String::from_utf8_lossy(&b).to_string()));
    }
}
