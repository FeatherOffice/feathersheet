//! O caso do usuário (17/10/2026): `=MÍNIMOSES(AO:AO;A:A;A2;C:C;C2;AP:AP;AP2)`
//! na coluna AU inteira do `2026_03.fshz` (13,7 milhões de linhas). Mede o
//! preencher em bloco, o cálculo de todas as linhas e a RAM.
//!
//! ```bash
//! cargo run --release --example medir_formula_coluna_inteira -- ~/Documentos/FeatherSheet/2026_03.fshz
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
    let caminho = std::env::args().nth(1).expect("passe o .fshz");
    let t = Instant::now();
    let lida = feathersheet_io::fshz_read::ler_fshz_arquivo(std::path::Path::new(&caminho)).expect("não abri");
    let mut pasta = feathersheet_io::para_workbook::para_workbook_soltando(lida);
    println!("abrir: {:?} · RSS {:.0} MB", t.elapsed(), mb("VmRSS:"));

    let s = &mut pasta.aba_mut(0).expect("aba").sheet;
    let ultima = s.max_row;
    let col = 46; // AU
    let t = Instant::now();
    s.set_cell(1, col, "=MÍNIMOSES(AO:AO;A:A;A2;C:C;C2;AP:AP;AP2)");
    let ok = s.faixa_vazia(col, 2, ultima)
        && s.preencher_formula_faixa_vazia(col, 2, ultima, "=MÍNIMOSES(AO:AO;A:A;A3;C:C;C3;AP:AP;AP3)");
    println!("preencher AU2:AU{}: {ok} · {:?} · RSS {:.0} MB", ultima + 1, t.elapsed(), mb("VmRSS:"));

    let t = Instant::now();
    let mut erros = 0u64;
    let mut soma = 0.0;
    for r in 1..=ultima {
        match s.get_value(r, col) {
            feathersheet_core::Value::Number(x) => soma += x,
            _ => erros += 1,
        }
        if r == 1 || r == 1_000_000 {
            println!("  linha {}: {:?} ({:?} até aqui)", r + 1, s.get_value(r, col), t.elapsed());
        }
    }
    println!("calcular {ultima} linhas: {:?} · soma {soma} · não-números {erros} · RSS {:.0} MB · pico {:.0} MB",
        t.elapsed(), mb("VmRSS:"), mb("VmHWM:"));
}
