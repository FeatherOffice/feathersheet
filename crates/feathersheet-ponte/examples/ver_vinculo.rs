//! Mostra o que uma pasta tem (abas, primeiras células) e as fórmulas com
//! vínculo externo: `cargo run --example ver_vinculo -- arquivo.fshz`.
use serde_json::Value;

fn main() {
    for caminho in std::env::args().skip(1) {
        let bytes = std::fs::read(&caminho).expect("ler");
        let mut p = feathersheet_ponte::Ponte::default();
        let r: Value = serde_json::from_slice(&p.abrir_fshz(&bytes).unwrap()).unwrap();
        println!("== {caminho}");
        for (i, aba) in r["estado"]["abas"].as_array().unwrap().iter().enumerate() {
            println!("aba {:?} ({}x{})", aba["nome"], aba["maxRow"], aba["maxCol"]);
            let s = &p.pasta.aba(i).unwrap().sheet;
            for row in 0..=aba["maxRow"].as_u64().unwrap().min(8) as u32 {
                let linha: Vec<String> = (0..=aba["maxCol"].as_u64().unwrap().min(9) as u32)
                    .map(|c| {
                        let raw = s.get_raw(row, c);
                        if raw.contains('[') { format!("{raw} => {:?}", p.pasta.valor(i, row, c)) } else { raw }
                    })
                    .collect();
                if linha.iter().any(|t| !t.is_empty()) {
                    println!("  {}: {:?}", row + 1, linha);
                }
            }
        }
    }
}
