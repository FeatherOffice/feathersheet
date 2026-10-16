//! Texto para Colunas pelo caminho em lote (análise em paralelo, gravação
//! sem foto por célula, desfazer leve), medido etapa por etapa.
//!
//! ```bash
//! cargo build --release --example medir_divisao_lote
//! systemd-run --user --scope -p MemoryMax=8G -p MemorySwapMax=0 \
//!   target/release/examples/medir_divisao_lote <csv>
//! ```

use feathersheet_core::operacoes::ops::{self, OpcoesDivisao, PlanoDivisao};
use feathersheet_core::planilha::workbook::Workbook;
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn pico_mb() -> f64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    status.lines().find_map(|l| l.strip_prefix("VmHWM:"))
        .and_then(|r| r.trim().trim_end_matches(" kB").trim().parse::<f64>().ok())
        .map_or(0.0, |kb| kb / 1024.0)
}

fn main() {
    let Some(caminho) = std::env::args().nth(1) else {
        eprintln!("uso: medir_divisao_lote <csv>");
        return;
    };
    let mut wb = Workbook::new();
    let mut n = 0u32;
    {
        let fonte = &mut wb.aba_mut(0).unwrap().sheet;
        let arquivo = std::fs::File::open(&caminho).expect("abrir CSV");
        for linha in BufReader::with_capacity(1 << 20, arquivo).lines() {
            fonte.set_cell_raw(n, 0, linha.expect("ler linha"), None);
            n += 1;
        }
        fonte.clear_history();
    }
    println!("[medição] carregado numa coluna: {n} linhas · pico {:.0} MB", pico_mb());
    let original = wb.aba(0).unwrap().sheet.get_raw(1, 0);
    let opcoes = OpcoesDivisao {
        delimitadores: vec![",".into()], consecutivos_como_um: false, qualificador_texto: Some('"'),
    };
    let colunas = ops::dividir_com_opcoes(&original, &opcoes).len() as u32;
    let plano = Arc::new(PlanoDivisao {
        col_origem: 0, r0: 0, r1: n - 1, dst_row: 0, dst_col: 0, colunas,
        largura: None, opcoes, tipos: Vec::new(), formulas: Default::default(),
    });

    let total = Instant::now();
    let s = &mut wb.aba_mut(0).unwrap().sheet;
    s.abrir_transacao();
    let t = Instant::now();
    let mut g = ops::iniciar_divisao(s, Arc::clone(&plano));
    println!("[medição] foto do desfazer (só o que existia): {:.2} s", t.elapsed().as_secs_f64());
    let (mut ler, mut preparar, mut gravar) = (Duration::ZERO, Duration::ZERO, Duration::ZERO);
    let mut r = 0u32;
    while r < n {
        let fim = (r + 16_384).min(n);
        let t = Instant::now();
        let textos: Vec<(u32, String)> = (r..fim).map(|l| (l, plano.texto_origem(s, l))).collect();
        ler += t.elapsed();
        let t = Instant::now();
        let linhas = plano.preparar_linhas(textos);
        preparar += t.elapsed();
        let t = Instant::now();
        s.gravar_divisao(&mut g, 0, &linhas);
        gravar += t.elapsed();
        r = fim;
    }
    s.fechar_transacao();
    println!("[medição] ler origem {:.2} s · cortar+analisar (paralelo) {:.2} s · gravar {:.2} s",
        ler.as_secs_f64(), preparar.as_secs_f64(), gravar.as_secs_f64());
    println!("[medição] TOTAL texto para colunas: {:.2} s · pico {:.0} MB · {} células",
        total.elapsed().as_secs_f64(), pico_mb(), s.len());

    let t = Instant::now();
    assert!(s.undo());
    println!("[medição] desfazer: {:.2} s · confere: {}",
        t.elapsed().as_secs_f64(), s.get_raw(1, 0) == original);
    let t = Instant::now();
    assert!(s.redo());
    println!("[medição] refazer: {:.2} s · pico {:.0} MB · {} células",
        t.elapsed().as_secs_f64(), pico_mb(), s.len());
    let t = Instant::now();
    let bytes = feathersheet_io::da_workbook::pasta_gravar_bytes(&wb, 0).expect("salvar");
    let lida = feathersheet_io::fshz_read::ler_fshz(&bytes).expect("reler");
    let volta = feathersheet_io::para_workbook::para_workbook(&lida);
    println!("[medição] salvar + reabrir: {:.2} s · {:.0} MB · A2 confere: {}",
        t.elapsed().as_secs_f64(), bytes.len() as f64 / 1048576.0,
        volta.aba(0).unwrap().sheet.get_raw(1, 0) == wb.aba(0).unwrap().sheet.get_raw(1, 0));
}
