//! Confere, célula a célula, que o CSV aberto pela ponte guarda o que o
//! arquivo tem: `cargo run --release --example conferir_csv -- arquivo.csv`.
use feathersheet_core::celulas::values::to_text;

fn main() {
    let caminho = std::env::args().nth(1).expect("o caminho do CSV");
    let texto = std::fs::read_to_string(&caminho).expect("ler");
    let esperado = feathersheet_io::csv::dividir_csv(&texto, ',');
    let mut p = feathersheet_ponte::Ponte::default();
    p.inserir_aba_de_csv("CSV".into(), &texto).unwrap();
    let i = p.pasta.len() - 1;
    let (mut erros, mut mostrados) = (0usize, 0);
    for (r, linha) in esperado.iter().enumerate() {
        for (c, campo) in linha.iter().enumerate() {
            let raw = p.pasta.aba(i).unwrap().sheet.get_raw(r as u32, c as u32);
            let valor = to_text(&p.pasta.valor(i, r as u32, c as u32));
            let mesmo_numero = matches!((campo.parse::<f64>(), valor.parse::<f64>()), (Ok(a), Ok(b)) if a == b);
            if raw != *campo && valor != *campo && !mesmo_numero {
                erros += 1;
                if mostrados < 8 {
                    mostrados += 1;
                    eprintln!("linha {r} coluna {c}: arquivo {campo:?}, raw {raw:?}, valor {valor:?}");
                }
            }
        }
    }
    eprintln!("{erros} células diferentes em {} linhas", esperado.len());
}
