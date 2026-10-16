//! Busca e ordenação na planilha do usuário (2 milhões de linhas):
//! `cargo run --release --example medir_busca_ordenar -- arquivo.fshz`.
use serde_json::{json, Value};
use std::time::Instant;

fn main() {
    let caminho = std::env::args().nth(1).expect("o caminho do .fshz");
    let bytes = std::fs::read(&caminho).expect("ler");
    let mut p = feathersheet_ponte::Ponte::default();
    let t = Instant::now();
    let r: Value = serde_json::from_slice(&p.abrir_fshz(&bytes).unwrap()).unwrap();
    drop(bytes);
    let aba = &r["estado"]["abas"][0];
    let id = aba["id"].as_u64().unwrap();
    eprintln!("aberto em {} ms: {} linhas × {} colunas", t.elapsed().as_millis(), aba["maxRow"], aba["maxCol"]);
    let mut chamar = |nome: &str, cmd: &str, corpo: Value| -> Value {
        let t = Instant::now();
        let saida: Value = serde_json::from_slice(&p.chamar(cmd, &serde_json::to_vec(&corpo).unwrap()).unwrap()).unwrap();
        eprintln!("{nome}: {} ms", t.elapsed().as_millis());
        saida
    };
    let opcoes = json!({ "matchCase": false, "wholeCell": false, "inFormulas": false });
    let r = chamar("localizar \"ALTO\" (500 + próxima)", "localizar", json!({ "aba": id, "consulta": "ALTO", "opcoes": opcoes, "limite": 500, "apos": { "linha": 0, "coluna": 0 } }));
    eprintln!("  total {}", r["total"]);
    chamar("de novo, \"Próxima\" (lista guardada)", "localizar", json!({ "aba": id, "consulta": "ALTO", "opcoes": opcoes, "limite": 500, "apos": { "linha": 5, "coluna": 5 } }));
    chamar("localizar \"normal\"", "localizar", json!({ "aba": id, "consulta": "normal", "opcoes": opcoes, "limite": 500, "apos": { "linha": 0, "coluna": 0 } }));
    chamar("localizar \"123\"", "localizar", json!({ "aba": id, "consulta": "123", "opcoes": opcoes, "limite": 500, "apos": { "linha": 0, "coluna": 0 } }));
    let formulas = json!({ "matchCase": false, "wholeCell": false, "inFormulas": true });
    let r = chamar("localizar \"s\" em fórmulas", "localizar", json!({ "aba": id, "consulta": "s", "opcoes": formulas, "limite": 500, "apos": { "linha": 0, "coluna": 0 } }));
    eprintln!("  total {}", r["total"]);
    let r = chamar("localizar \"s\" nos valores", "localizar", json!({ "aba": id, "consulta": "s", "opcoes": opcoes, "limite": 500, "apos": { "linha": 0, "coluna": 0 } }));
    eprintln!("  total {}", r["total"]);
    if std::env::var("SEM_ORDENAR").is_ok() {
        return;
    }
    let area = json!({ "r0": 0, "c0": 0, "r1": aba["maxRow"], "c1": aba["maxCol"] });
    let (ultima, colunas) = (aba["maxRow"].as_u64().unwrap() as u32, aba["maxCol"].as_u64().unwrap() as u32);
    let linha = |p: &feathersheet_ponte::Ponte, r: u32| -> Vec<String> {
        (0..=colunas).map(|c| format!("{:?} {}", p.pasta.valor(0, r, c), p.pasta.aba(0).unwrap().sheet.get_raw(r, c))).collect()
    };
    let (primeira_antes, ultima_antes) = (linha(&p, 1), linha(&p, ultima));
    let mut chamar = |nome: &str, cmd: &str, corpo: Value| {
        let t = Instant::now();
        p.chamar(cmd, &serde_json::to_vec(&corpo).unwrap()).unwrap();
        eprintln!("{nome}: {} ms", t.elapsed().as_millis());
    };
    chamar("ordenar Z→A pela coluna A, com cabeçalho", "ordenar", json!({ "aba": id, "area": area, "cabecalho": true, "chaves": [{ "coluna": 0, "crescente": false }] }));
    let depois = linha(&p, 1);
    eprintln!("  a última linha subiu para a 2, igual (valores e texto das fórmulas deslocado)?");
    for (a, b) in ultima_antes.iter().zip(&depois) {
        eprintln!("    antes na {}: {a}\n    agora na 2: {b}", ultima + 1);
    }
    let t = Instant::now();
    p.chamar("desfazer", &serde_json::to_vec(&json!({ "aba": id })).unwrap()).unwrap();
    eprintln!("desfazer: {} ms", t.elapsed().as_millis());
    eprintln!("  a linha 2 voltou? {}", linha(&p, 1) == primeira_antes);
}
