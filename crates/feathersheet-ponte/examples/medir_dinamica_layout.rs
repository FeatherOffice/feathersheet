//! A dinâmica na planilha do usuário, layout antigo contra o compacto com
//! subtotais e dois níveis: `cargo run --release --example
//! medir_dinamica_layout -- planilha_2m.plz`.
use serde_json::{json, Value};
use std::time::Instant;

fn main() {
    let caminho = std::env::args().nth(1).expect("o caminho do .fshz");
    let bytes = std::fs::read(&caminho).expect("ler");
    let mut p = feathersheet_ponte::Ponte::default();
    let r: Value = serde_json::from_slice(&p.abrir_fshz(&bytes).unwrap()).unwrap();
    let aba = &r["estado"]["abas"][0];
    let id = aba["id"].as_u64().unwrap();
    let (max_r, max_c) = (aba["maxRow"].as_u64().unwrap(), aba["maxCol"].as_u64().unwrap());
    let cab: Vec<String> = (0..=max_c).map(|c| p.pasta.aba(0).unwrap().sheet.get_raw(0, c as u32)).collect();
    eprintln!("{max_r} linhas; cabeçalho {cab:?}");
    let linhas: Vec<u64> = std::env::var("LINHAS").map(|s| s.split(',').map(|x| x.parse().unwrap()).collect()).unwrap_or(vec![1, 2]);
    let valor: u64 = std::env::var("VALOR").ok().and_then(|s| s.parse().ok()).unwrap_or(3);
    for (nome, opcoes) in [
        ("antigo", json!(null)),
        ("compacto+subtotais", json!({ "layout": "compacto", "subtotais": true })),
        ("tabular sem repetir", json!({ "layout": "tabular", "repetir": false, "subtotais": true })),
    ] {
        let t = Instant::now();
        let r = p.chamar(
            "dinamica_aplicar",
            &serde_json::to_vec(&json!({
                "aba": id, "origemAba": id, "linha": 0, "coluna": max_c + 3,
                "limpar": { "r0": 0, "c0": max_c + 3, "r1": 5000, "c1": max_c + 20 },
                "config": { "origem": { "r0": 0, "c0": 0, "r1": max_r, "c1": max_c }, "linhas": linhas, "colunas": [],
                            "valores": [{ "coluna": valor, "op": "soma" }], "opcoes": opcoes }
            }))
            .unwrap(),
        );
        let r: Value = serde_json::from_slice(&r.unwrap()).unwrap();
        eprintln!("{nome}: {} ms, {} linhas no resultado", t.elapsed().as_millis(), r["altura"]);
    }
}
