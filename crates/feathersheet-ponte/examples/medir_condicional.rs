//! O custo de um quadro da formatação condicional num CSV grande:
//! `cargo run --release --example medir_condicional -- arquivo.csv`.
use serde_json::json;

fn main() {
    let caminho = std::env::args().nth(1).expect("o caminho do CSV");
    let texto = std::fs::read_to_string(&caminho).expect("ler");
    let mut p = feathersheet_ponte::Ponte::default();
    let r: serde_json::Value = serde_json::from_slice(&p.inserir_aba_de_csv("CSV".into(), &texto).unwrap()).unwrap();
    drop(texto);
    let (id, linhas) = (r["id"].as_u64().unwrap(), r["linhas"].as_u64().unwrap());
    let regras = json!([
        { "id": "esc", "ranges": [{ "r0": 1, "c0": 4, "r1": 1_048_575, "c1": 4 }],
          "condition": { "kind": "colorScale", "min": { "type": "min", "color": "#f8696b" }, "max": { "type": "max", "color": "#63be7b" } } },
        { "id": "alt", "ranges": [{ "r0": 0, "c0": 0, "r1": 1_048_575, "c1": 19 }],
          "condition": { "kind": "cell", "op": "formula", "values": ["=ÉPAR(LIN())"], "style": { "bg": "#f3f3f3" } } },
    ]);
    p.chamar("metadados_definir", &serde_json::to_vec(&json!({ "aba": id, "r": regras })).unwrap()).unwrap();
    let quadro = |p: &mut feathersheet_ponte::Ponte, r0: u64| {
        let t = std::time::Instant::now();
        let corpo = json!({ "aba": id, "r0": r0, "c0": 0, "r1": r0 + 40, "c1": 19 });
        let n = p.chamar("condicional_bloco", &serde_json::to_vec(&corpo).unwrap()).unwrap().len();
        (t.elapsed().as_micros(), n)
    };
    eprintln!("{linhas} linhas; quadro de 41 × 20 células");
    eprintln!("primeiro quadro (monta a estatística da escala): {:?} µs, bytes", quadro(&mut p, 0));
    eprintln!("quadro seguinte, rolado: {:?} µs, bytes", quadro(&mut p, 500_000));
    eprintln!("quadro seguinte, rolado: {:?} µs, bytes", quadro(&mut p, 1_000_000));
    let corpo = json!({ "aba": id, "linha": 5, "coluna": 4, "raw": "99" });
    p.chamar("celula_gravar", &serde_json::to_vec(&corpo).unwrap()).unwrap();
    eprintln!("depois de editar uma célula (refaz a escala): {:?} µs, bytes", quadro(&mut p, 1_000_000));
}
