//! Teste do desfazer "por versão" (o jeito do Git interno) no Texto para
//! Colunas, contra o histórico célula por célula.
//!
//! Em vez de fotografar cada célula escrita, guarda a pasta inteira como
//! `.fshz` em memória ANTES da operação (a "versão"), divide sem histórico,
//! e desfazer = abrir aquela versão de volta. Refazer precisaria de uma
//! segunda versão, gravada depois da operação — também medida aqui.
//!
//! ```bash
//! cargo build --release --example medir_versao_divisao
//! systemd-run --user --scope -p MemoryMax=8G -p MemorySwapMax=0 \
//!   target/release/examples/medir_versao_divisao <csv>
//! ```

use feathersheet_core::operacoes::ops::{self, OpcoesDivisao};
use feathersheet_core::planilha::workbook::Workbook;
use std::io::{BufRead, BufReader};
use std::time::Instant;

fn pico_mb() -> f64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    status.lines().find_map(|l| l.strip_prefix("VmHWM:"))
        .and_then(|r| r.trim().trim_end_matches(" kB").trim().parse::<f64>().ok())
        .map_or(0.0, |kb| kb / 1024.0)
}

fn main() {
    let Some(caminho) = std::env::args().nth(1) else {
        eprintln!("uso: medir_versao_divisao <csv>");
        return;
    };
    let t = Instant::now();
    let mut wb = Workbook::new();
    let mut n = 0u32;
    {
        let fonte = &mut wb.aba_mut(0).unwrap().sheet;
        let arquivo = std::fs::File::open(&caminho).expect("abrir CSV");
        for linha in BufReader::with_capacity(1 << 20, arquivo).lines() {
            fonte.set_cell_raw(n, 0, linha.expect("ler linha"), None);
            n += 1;
        }
    }
    println!("[medição] carregar numa coluna: {n} linhas · {:.1} s · pico {:.0} MB",
        t.elapsed().as_secs_f64(), pico_mb());
    let original = wb.aba(0).unwrap().sheet.get_raw(1, 0);

    let t = Instant::now();
    let antes = feathersheet_io::da_workbook::pasta_gravar_bytes(&wb, 0).expect("versão antes");
    println!("[medição] versão antes (gravar .fshz em memória): {:.2} s · {:.0} MB",
        t.elapsed().as_secs_f64(), antes.len() as f64 / 1048576.0);

    let opcoes = OpcoesDivisao {
        delimitadores: vec![",".into()],
        consecutivos_como_um: false,
        qualificador_texto: Some('"'),
    };
    let t = Instant::now();
    let mut inicio = 0u32;
    while inicio < n {
        let fim = (inicio + 2048).min(n);
        let linhas: Vec<(u32, Vec<String>)> = {
            let s = &wb.aba(0).unwrap().sheet;
            (inicio..fim).map(|r| (r, ops::dividir_com_opcoes(&s.get_raw(r, 0), &opcoes))).collect()
        };
        let s = &mut wb.aba_mut(0).unwrap().sheet;
        for (r, partes) in &linhas {
            for (c, parte) in partes.iter().enumerate() {
                s.set_cell_divisao(*r, c as u32, parte.trim());
            }
        }
        inicio = fim;
    }
    println!("[medição] dividir SEM histórico: {:.1} s · pico {:.0} MB",
        t.elapsed().as_secs_f64(), pico_mb());

    let t = Instant::now();
    let depois = feathersheet_io::da_workbook::pasta_gravar_bytes(&wb, 0).expect("versão depois");
    println!("[medição] versão depois (para o refazer): {:.2} s · {:.0} MB",
        t.elapsed().as_secs_f64(), depois.len() as f64 / 1048576.0);

    let t = Instant::now();
    let lida = feathersheet_io::fshz_read::ler_fshz(&antes).expect("ler versão antes");
    let volta = feathersheet_io::para_workbook::para_workbook(&lida);
    let restaurado = volta.aba(0).unwrap().sheet.get_raw(1, 0);
    println!("[medição] desfazer = abrir a versão antes: {:.2} s · confere: {}",
        t.elapsed().as_secs_f64(), restaurado == original);
    drop((lida, volta));

    let t = Instant::now();
    let lida = feathersheet_io::fshz_read::ler_fshz(&depois).expect("ler versão depois");
    let _refeito = feathersheet_io::para_workbook::para_workbook(&lida);
    println!("[medição] refazer = abrir a versão depois: {:.2} s · pico {:.0} MB",
        t.elapsed().as_secs_f64(), pico_mb());
}
