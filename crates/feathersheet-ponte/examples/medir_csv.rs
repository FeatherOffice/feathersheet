//! Mede a abertura de um CSV pela ponte: `cargo run --release --example
//! medir_csv -- arquivo.csv [antigo]`. Rodar com `/usr/bin/time -v` para ver
//! o pico (`Maximum resident set size`).
fn main() {
    let mut args = std::env::args().skip(1);
    let caminho = args.next().expect("o caminho do CSV");
    let antigo = args.next().as_deref() == Some("antigo");
    let texto = std::fs::read_to_string(&caminho).expect("ler");
    let mut ponte = feathersheet_ponte::Ponte::default();
    let inicio = std::time::Instant::now();
    if antigo {
        let linhas = feathersheet_io::csv::dividir_csv(&texto, feathersheet_io::csv::detectar_separador(&texto));
        ponte.inserir_aba_de_linhas("CSV".into(), &linhas).unwrap();
    } else {
        ponte.inserir_aba_de_csv("CSV".into(), &texto).unwrap();
    }
    eprintln!("{} ms", inicio.elapsed().as_millis());
}
