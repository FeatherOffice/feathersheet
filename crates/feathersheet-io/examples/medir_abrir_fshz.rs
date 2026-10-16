//! Quanto custa abrir um `.fshz` direto na memória do Rust — o caminho do
//! comando `pasta_abrir`, que é por onde a ponte começa.
//!
//! ```bash
//! cargo run --release --example medir_abrir_plz -- ~/Downloads/planilha_2m.plz
//! ```

use std::time::Instant;

fn mb(campo: &str) -> f64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    status
        .lines()
        .find_map(|l| l.strip_prefix(campo))
        .and_then(|r| r.trim().trim_end_matches(" kB").trim().parse::<f64>().ok())
        .map_or(0.0, |kb| kb / 1024.0)
}

fn main() {
    let caminho = std::env::args().nth(1).expect("passe o caminho do .fshz");
    let t = Instant::now();
    let bytes = std::fs::read(&caminho).expect("não li o arquivo");
    let lida = feathersheet_io::fshz_read::ler_fshz(&bytes).expect("não entendi o arquivo");
    println!("ler_fshz: {:?} · RSS {:.0} MB · pico {:.0} MB", t.elapsed(), mb("VmRSS:"), mb("VmHWM:"));
    for aba in &lida.sheets {
        println!("aba {}: {} avulsas", aba.n, aba.avulsas.len());
        for c in &aba.colunas_num { println!("  num   col {} ({} linhas)", c.col, c.values.len()); }
        for c in &aba.colunas_txt { println!("  texto col {} ({} linhas, dict {:?}…)", c.col, c.indices.len(), c.dict.iter().take(3).collect::<Vec<_>>()); }
        for c in &aba.colunas_formula {
            let slot = c.slots.get(1).copied().unwrap_or(0) as usize;
            let padrao = aba.padroes_formula.get(slot).cloned().unwrap_or_default();
            println!("  fórm. col {} ({} linhas) padrão da linha 2: {:?} -> {:?}", c.col, c.slots.len(), padrao,
                feathersheet_core::formula::formula_pattern::expand_pattern(&padrao, 1, c.col as i32));
        }
    }
    let t = Instant::now();
    let pasta = feathersheet_io::para_workbook::para_workbook(&lida);
    drop(lida);
    drop(bytes);
    println!("para_workbook: {:?} · RSS {:.0} MB · pico {:.0} MB", t.elapsed(), mb("VmRSS:"), mb("VmHWM:"));
    let aba = &pasta.aba(0).unwrap().sheet;
    let usado = aba.used_rect();
    println!("aba {:?}: usado {}..{} x {}..{}", aba.name, usado.r0, usado.r1, usado.c0, usado.c1);
    for c in 0..=usado.c1 {
        println!("  coluna {c}: linha 2 = {:?}  bruto {:?}", aba.get_value(1, c), aba.get_raw(1, c));
    }
    let t = Instant::now();
    let mut soma = 0.0;
    let mut n = 0u32;
    for r in 1..=usado.r1 {
        if let feathersheet_core::Value::Number(v) = aba.get_value(r, 4) {
            soma += v;
            n += 1;
        }
    }
    println!("soma da coluna E ({n} números = {soma:.2}): {:?} · RSS {:.0} MB · pico {:.0} MB", t.elapsed(), mb("VmRSS:"), mb("VmHWM:"));
}
