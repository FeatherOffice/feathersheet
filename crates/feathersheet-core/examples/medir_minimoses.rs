//! O caso que travou o app em 17/10/2026: `=MÍNIMOSES(AO:AO;A:A;A2;C:C;C2;
//! AP:AP;AP2)` preenchida numa coluna inteira. Mede o preencher (Ctrl+D) e o
//! cálculo para alguns tamanhos: se dobrar as linhas quadruplica o tempo, é
//! O(n²).
//!
//! ```bash
//! cargo run --release --example medir_minimoses -- 20000 40000 80000
//! ```

use std::time::Instant;
use feathersheet_core::celulas::refs::Rect;
use feathersheet_core::planilha::sheet::Sheet;

fn main() {
    for n in std::env::args().skip(1).map(|a| a.parse::<u32>().expect("número de linhas")) {
        let mut s = Sheet::new("Dados");
        s.set_cell(0, 0, "RAMAL");
        s.set_cell(0, 2, "LINHA");
        s.set_cell(0, 40, "_KM");
        s.set_cell(0, 41, "NOMENCLATURA");
        for r in 1..=n {
            let grupo = r / 500;
            s.set_cell(r, 0, &format!("R{}", grupo % 7));
            s.set_cell(r, 2, &format!("L{}", grupo % 14));
            s.set_cell(r, 41, &format!("EH {grupo:02}"));
            s.set_cell(r, 40, &format!("{}", 1000 + r));
        }
        s.set_cell(1, 46, "=MÍNIMOSES(AO:AO;A:A;A2;C:C;C2;AP:AP;AP2)");
        let t = Instant::now();
        s.preencher(&Rect::new(1, 46, 1, 46), &Rect::new(1, 46, n, 46));
        let preencher = t.elapsed();
        let t = Instant::now();
        let mut soma = 0.0;
        for r in 1..=n {
            if let feathersheet_core::celulas::values::Value::Number(x) = s.get_value(r, 46) { soma += x; }
        }
        println!("{n} linhas: preencher {preencher:?} · ler os resultados {:?} · soma {soma}", t.elapsed());
    }
}
