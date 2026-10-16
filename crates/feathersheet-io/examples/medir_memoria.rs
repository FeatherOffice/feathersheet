//! Bancada de memória (plano de 400 MB, Fase 0): RAM e pico em cada etapa
//! de abrir um `.fshz` ou `.xlsx`, e quanto cada estrutura da aba guarda.
//!
//! cargo run --release --example medir_memoria -- ARQUIVO [--calcular]
use std::time::Instant;

/// (RSS atual, pico desde o início) em MB, do `/proc/self/status`.
fn ram() -> (u64, u64) {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let campo = |nome: &str| status.lines().find(|l| l.starts_with(nome))
        .and_then(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok()).unwrap_or(0) / 1024;
    (campo("VmRSS:"), campo("VmHWM:"))
}

fn etapa(nome: &str, desde: Instant) {
    let (atual, pico) = ram();
    println!("[{nome}] {:.1} s · {atual} MB · pico {pico} MB", desde.elapsed().as_secs_f64());
}

fn main() {
    let caminho = std::env::args().nth(1).expect("informe o arquivo");
    let calcular = std::env::args().any(|a| a == "--calcular");
    if let Some(n) = std::env::var("NUCLEOS").ok().and_then(|n| n.parse().ok()) {
        feathersheet_core::paralelismo::definir(n);
    }
    etapa("início", Instant::now());
    let t = Instant::now();
    let caminho = std::path::Path::new(&caminho);
    let mut w = if caminho.extension().is_some_and(|e| e == "fshz") {
        let lida = feathersheet_io::fshz_read::ler_fshz_arquivo(caminho).expect("ler");
        etapa("fshz lido (estrutura intermediária)", t);
        for aba in &lida.sheets {
            println!("  aba {:?}: {} estilos por célula · {} avulsas · {} col. número · {} col. texto · {} col. fórmula",
                aba.n, aba.estilos_diretos.len(), aba.avulsas.len(), aba.colunas_num.len(),
                aba.colunas_txt.len(), aba.colunas_formula.len());
        }
        let w = feathersheet_io::para_workbook::para_workbook(&lida);
        etapa("planilha montada", t);
        drop(lida);
        etapa("estrutura intermediária solta", t);
        w
    } else {
        let (w, _) = feathersheet_io::xlsx_read::ler_xlsx_arquivo(caminho).expect("ler");
        etapa("xlsx aberto", t);
        w
    };
    for linha in feathersheet_core::diario::recolher() {
        println!("  {linha}");
    }
    if calcular {
        let t = Instant::now();
        for i in 0..w.nomes().len() {
            let u = w.aba(i).unwrap().sheet.used_rect();
            for r in u.r0..=u.r1.min(40) {
                for c in u.c0..=u.c1 {
                    let _ = w.valor(i, r, c);
                }
            }
        }
        etapa("primeiras 40 linhas de cada aba calculadas", t);
        w.soltar_faixas();
        while w.consolidar_calculos(usize::MAX) > 0 {}
        etapa("faixas soltas e resultados nas colunas", t);
    }
    for (i, nome) in w.nomes().iter().enumerate() {
        println!("  {nome}: {}", w.aba(i).unwrap().sheet.contagem_memoria());
    }
}
