//! A dinâmica e o gráfico sobre um CSV grande, pela ponte: `cargo run
//! --release --example medir_dinamica_grafico -- arquivo.csv`. O CSV precisa
//! de 20 colunas no formato do exemplo do Spotify (gênero na 20ª).
use serde_json::{json, Value as Json};

fn chamar(p: &mut feathersheet_ponte::Ponte, cmd: &str, corpo: Json) -> Json {
    let t = std::time::Instant::now();
    let bytes = p.chamar(cmd, &serde_json::to_vec(&corpo).unwrap()).unwrap();
    eprintln!("  {cmd}: {} ms", t.elapsed().as_millis());
    serde_json::from_slice(&bytes).unwrap_or(Json::Null)
}

fn main() {
    let caminho = std::env::args().nth(1).expect("o caminho do CSV");
    let texto = std::fs::read_to_string(&caminho).expect("ler");
    let mut p = feathersheet_ponte::Ponte::default();
    let r: Json = serde_json::from_slice(&p.inserir_aba_de_csv("CSV".into(), &texto).unwrap()).unwrap();
    drop(texto);
    let (origem, linhas) = (r["id"].as_u64().unwrap(), r["linhas"].as_u64().unwrap());
    let destino = chamar(&mut p, "aba_inserir", json!({ "nome": "Dinâmica" }))["id"].as_u64().unwrap();
    let area = json!({ "r0": 0, "c0": 0, "r1": linhas - 1, "c1": 19 });
    for (nome, col) in [("gênero (114)", 19), ("artista (50 mil)", 1), ("id (1,3 milhão)", 0)] {
        eprintln!("dinâmica por {nome}, soma da popularidade:");
        let r = chamar(&mut p, "dinamica_aplicar", json!({
            "aba": destino, "origemAba": origem, "linha": 0, "coluna": 0,
            "limpar": { "r0": 0, "c0": 0, "r1": 1_048_575, "c1": 10 },
            "config": { "origem": area, "linhas": [col], "colunas": [], "valores": [{ "coluna": 4, "op": "soma" }] },
        }));
        eprintln!("  -> {} linhas", r["altura"]);
    }
    for (nome, agregar) in [("colunas, 2 séries, sem agrupar", false), ("agrupado por gênero", true)] {
        eprintln!("gráfico {nome}:");
        let cfg = json!({ "id": "g", "type": "column", "title": "", "range": { "r0": 0, "c0": 17, "r1": linhas - 1, "c1": 19 },
            "seriesInColumns": true, "hasHeaders": true, "hasCategoryLabels": true, "categoryColumnIndex": 19, "aggregate": agregar });
        let d = chamar(&mut p, "grafico_dados", json!({ "aba": origem, "config": cfg }));
        eprintln!("  -> {} categorias", d["categories"].as_array().map_or(0, |a| a.len()));
    }
}
