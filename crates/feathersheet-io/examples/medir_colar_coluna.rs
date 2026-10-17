//! Colar um valor da linha 2 até a última, como o Ctrl+V da interface faz
//! com texto de fora (um `set_cell` por célula numa transação):
//! `medir_colar_coluna ARQUIVO.csv COLUNA`.
use std::time::Instant;

fn main() {
    let caminho = std::path::PathBuf::from(std::env::args().nth(1).expect("arquivo"));
    let col: u32 = std::env::args().nth(2).and_then(|c| c.parse().ok()).unwrap_or(0);
    let mut w = feathersheet_io::csv_workbook::importar_csv_tsv_arquivo(&caminho, false, Some(',')).expect("abrir");
    let ultima = w.aba(0).unwrap().sheet.max_row;
    println!("[aberto] última linha {ultima}");
    let t = Instant::now();
    w.editar(0, |s| {
        s.abrir_transacao();
        for r in 1..=ultima {
            s.set_cell(r, col, "5");
        }
        s.fechar_transacao();
    });
    println!("[colar] {} células na coluna {col} em {:.2} s", ultima, t.elapsed().as_secs_f64());
}
