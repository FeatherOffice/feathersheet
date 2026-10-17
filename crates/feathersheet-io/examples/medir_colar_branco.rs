//! Colar um valor na coluna A inteira de uma planilha em branco, como o
//! Ctrl+V (um `set_cell` por célula numa transação): `medir_colar_branco VALOR LINHAS`.
use std::time::Instant;

fn main() {
    let valor = std::env::args().nth(1).unwrap_or_else(|| "5".into());
    let linhas: u32 = std::env::args().nth(2).and_then(|n| n.parse().ok()).unwrap_or(1_048_576);
    let mut w = feathersheet_core::planilha::workbook::Workbook::new();
    let t = Instant::now();
    w.editar(0, |s| {
        s.abrir_transacao();
        for r in 0..linhas {
            s.set_cell(r, 0, &valor);
        }
        s.fechar_transacao();
    });
    let s = &w.aba(0).unwrap().sheet;
    println!("[colar {valor:?}] {linhas} linhas em {:.2} s · estilos {} · {:?}", t.elapsed().as_secs_f64(), s.styles.len(), s.get_value(3, 0));
}
