//! O custo das operações por linha (passo 3) numa planilha grande, gerada
//! aqui: `cargo run --release --example medir_ops_linhas -- 500000`.
//! Mede ordenar, remover repetidas, aparar e desfazer, com o pico de memória
//! (`/usr/bin/time -v` por fora).
use serde_json::{json, Value};

fn main() {
    let linhas: usize = std::env::args().nth(1).and_then(|n| n.parse().ok()).unwrap_or(200_000);
    let mut csv = String::from("id;nome;cidade;valor;obs\n");
    for i in 0..linhas {
        let cidade = ["Rio", "SP", "BH", "Recife", "  Porto   Alegre "][i % 5];
        csv.push_str(&format!("{};nome{};{};{};o{}\n", (i * 7919) % linhas, i % 1000, cidade, i % 97, i));
    }
    let mut p = feathersheet_ponte::Ponte::default();
    let r: Value = serde_json::from_slice(&p.inserir_aba_de_csv("CSV".into(), &csv).unwrap()).unwrap();
    drop(csv);
    let id = r["id"].as_u64().unwrap();
    let area = json!({ "r0": 0, "c0": 0, "r1": 9_999_999, "c1": 4 });
    let mut medir = |nome: &str, cmd: &str, corpo: Value| {
        let t = std::time::Instant::now();
        let saida: Value = serde_json::from_slice(&p.chamar(cmd, &serde_json::to_vec(&corpo).unwrap()).unwrap()).unwrap();
        eprintln!("{nome}: {} ms (total {})", t.elapsed().as_millis(), saida["total"]);
    };
    eprintln!("{linhas} linhas × 5 colunas");
    medir("ordenar pelo id", "ordenar", json!({ "aba": id, "area": area, "cabecalho": true, "chaves": [{ "coluna": 0 }] }));
    medir("desfazer", "desfazer", json!({ "aba": id }));
    medir("remover repetidas por nome", "remover_repetidas", json!({ "aba": id, "area": area, "cabecalho": true, "colunas": [1] }));
    medir("desfazer", "desfazer", json!({ "aba": id }));
    medir("aparar espaços", "aparar_espacos", json!({ "aba": id, "area": area }));
}
