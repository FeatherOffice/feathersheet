//! A alça de preenchimento (duplo clique) e o motor da tabela numa planilha
//! grande: `cargo run --release --example medir_alca_tabela -- planilha_2m.csv`
//! (sem argumento, gera 2 milhões de linhas aqui). Pico de memória com
//! `/usr/bin/time -v` por fora.
//!
//! O "antes" do duplo clique é o que o `fillHandleDoubleClick` pedia à ponte
//! com a planilha no Rust: um `bloco` de 64 × 32 com texto a cada 64 linhas,
//! até o fim da coluna vizinha. Aqui mede só o lado Rust desses pedidos — no
//! aplicativo cada um ainda atravessa o IPC e vira objeto no JavaScript, então
//! é um piso do que custava.
use serde_json::{json, Value};
use std::time::Instant;

fn main() {
    let texto = match std::env::args().nth(1) {
        Some(caminho) => std::fs::read_to_string(&caminho).expect("ler o CSV"),
        None => {
            let mut csv = String::from("ID;ValorA;ValorB;Soma;Status\n");
            for i in 0..2_000_000u32 {
                csv.push_str(&format!("{};{};{};=B{}+C{};{}\n", i + 1, i % 997, i % 13, i + 2, i + 2, ["ALTO", "NORMAL"][(i % 3 == 0) as usize]));
            }
            csv
        }
    };
    let mut p = feathersheet_ponte::Ponte::default();
    let t = Instant::now();
    let r: Value = serde_json::from_slice(&p.inserir_aba_de_csv("CSV".into(), &texto).unwrap()).unwrap();
    drop(texto);
    let id = r["id"].as_u64().unwrap();
    let estado: Value = serde_json::from_slice(&p.chamar("pasta_estado", b"{}").unwrap()).unwrap();
    let aba = estado["abas"].as_array().unwrap().iter().find(|a| a["id"] == id).unwrap().clone();
    let (ultima, colunas) = (aba["maxRow"].as_u64().unwrap() as u32, aba["maxCol"].as_u64().unwrap() as u32);
    eprintln!("aberto em {} ms: {} linhas × {} colunas", t.elapsed().as_millis(), ultima + 1, colunas + 1);

    let mut chamar = |nome: &str, cmd: &str, corpo: Value| -> Value {
        let t = Instant::now();
        let bytes = p.chamar(cmd, &serde_json::to_vec(&corpo).unwrap()).unwrap();
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        let saida: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        eprintln!("{nome}: {ms:.1} ms");
        saida
    };

    // A coluna logo à direita dos dados ganha uma fórmula na linha 2; o duplo
    // clique nela desce até o fim da coluna vizinha.
    let alvo = colunas + 1;
    chamar("gravar a fórmula", "celula_gravar", json!({ "aba": id, "linha": 1, "coluna": alvo, "raw": "=A2*2" }));

    // Antes: os blocos que o TypeScript pedia (`hasContent`/`getRaw` linha a
    // linha, um bloco novo a cada 64 linhas), sem contar o IPC.
    let t = Instant::now();
    let mut blocos = 0u32;
    let c0 = alvo - 1 - ((alvo - 1) % 32);
    for r0 in (0..=ultima + 100).step_by(64) {
        let _ = p.chamar("bloco", &serde_json::to_vec(&json!({ "aba": id, "r0": r0, "c0": c0, "r1": r0 + 63, "c1": c0 + 31, "texto": true })).unwrap());
        blocos += 1;
    }
    eprintln!("ANTES — duplo clique pelo TypeScript: {blocos} blocos pela ponte, {:.0} ms só do lado Rust", t.elapsed().as_secs_f64() * 1000.0);

    let mut chamar = |nome: &str, cmd: &str, corpo: Value| -> Value {
        let t = Instant::now();
        let saida: Value = serde_json::from_slice(&p.chamar(cmd, &serde_json::to_vec(&corpo).unwrap()).unwrap()).unwrap_or(Value::Null);
        eprintln!("{nome}: {:.1} ms", t.elapsed().as_secs_f64() * 1000.0);
        saida
    };
    let sel = json!({ "r0": 1, "c0": alvo, "r1": 1, "c1": alvo });
    let r = chamar("DEPOIS — só achar o fim (planejar)", "preencher_ate_o_fim", json!({ "aba": id, "selecao": sel, "colunas": 16384, "planejar": true }));
    eprintln!("  destino {}", r["destino"]);
    let r = chamar("DEPOIS — duplo clique inteiro (achar + preencher)", "preencher_ate_o_fim", json!({ "aba": id, "selecao": sel, "colunas": 16384 }));
    eprintln!("  {} células", r["total"]);
    chamar("desfazer", "desfazer", json!({ "aba": id }));

    // A tabela: sugerida (Ctrl+T), total da coluna, crescer.
    let r = chamar("tabela sugerida (Ctrl+T) a partir de A1", "tabela_sugerida", json!({ "aba": id, "linha": 0, "coluna": 0, "usados": ["Tabela1"] }));
    eprintln!("  {} cabeçalho={} nome={}", r["area"], r["temCabecalho"], r["nome"]);
    let coluna_b = json!({ "r0": 1, "c0": 1, "r1": ultima, "c1": 1 });
    for f in ["soma", "contagem", "desvio"] {
        let r = chamar(&format!("total da coluna B ({f})"), "tabela_total", json!({ "aba": id, "area": coluna_b, "funcao": f }));
        eprintln!("  = {r}");
    }
    let r = chamar("total com texto formatado (o que o desenho pede)", "tabela_total", json!({ "aba": id, "area": coluna_b, "funcao": "soma", "formatarEm": { "row": ultima, "col": 1 } }));
    eprintln!("  = {}", r["texto"]);
    let tabelas = json!([{ "area": { "r0": 0, "c0": 0, "r1": ultima, "c1": colunas }, "mostrarTotais": false }]);
    let r = chamar("crescer (digitou logo abaixo)", "tabela_crescer", json!({ "aba": id, "linha": ultima + 1, "coluna": 0, "tabelas": tabelas }));
    eprintln!("  {r}");
}
