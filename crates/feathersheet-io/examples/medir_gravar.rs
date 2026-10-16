//! Medição da gravação do .fshz a partir da Workbook nativa em Rust.

use std::time::Instant;
use feathersheet_core::planilha::workbook::Workbook;

fn main() {
    let mut wb = Workbook::new();
    {
        let viva = wb.aba_mut(0).unwrap();
        for r in 0..100_000 {
            viva.sheet.set_cell_raw(r, 0, format!("{}", r * 2), None);
            viva.sheet.set_cell_raw(r, 1, format!("Texto {}", r), None);
        }
    }

    let t0 = Instant::now();
    let bytes = feathersheet_io::da_workbook::pasta_gravar_bytes(&wb, 0).expect("gravação falhou");
    let dur = t0.elapsed();

    println!(
        "Gravação de 200.000 células em Rust: {:.2} ms · {:.2} MB compactado",
        dur.as_secs_f64() * 1000.0,
        bytes.len() as f64 / (1024.0 * 1024.0)
    );
}
