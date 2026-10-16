//! Smoke executável da migração IO-006: parser compartilhado em uso real no Sheet.
//! cargo run --manifest-path feathersheet/crates/feathersheet-io/Cargo.toml --example validacao_csv_lotes -j 2

use feathersheet_io::csv::{detectar_separador, dividir_csv_em_lotes};

fn main() {
    let entrada = "\u{feff}Nome;Valor;Observação\r\nJoão;1.234,56;\"linha\r\ndois\"\r\nAna;-0,25;\"A\"\"B\"\r\n";
    let separador = detectar_separador(entrada);
    assert_eq!(separador, ';');
    let mut resultado = Vec::<Vec<String>>::new();
    let mut lotes = 0;
    dividir_csv_em_lotes(entrada, separador, 1, |linhas| {
        assert_eq!(linhas.len(), 1, "cada lote deve ter uma linha");
        lotes += 1;
        resultado.append(linhas);
    });
    assert_eq!(
        resultado,
        vec![
            vec!["Nome", "Valor", "Observação"],
            vec!["João", "1.234,56", "linha\r\ndois"],
            vec!["Ana", "-0,25", "A\"B"],
        ]
    );
    assert_eq!(lotes, 3);
    println!("IO-006 OK: 3 linhas, 3 lotes; BOM, CRLF, aspas, multilinha e locale preservados.");
}
