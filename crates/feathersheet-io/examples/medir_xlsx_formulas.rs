//! Mede a abertura de um XLSX com muitas fórmulas e o custo de calcular
//! partes dele, do mais leve ao mais pesado.
//!
//! cargo run --release --example medir_xlsx_formulas -- ARQUIVO.xlsx
use std::time::Instant;

use feathersheet_io::xlsx_read::ler_xlsx_arquivo;

fn rss_mb() -> u64 {
    std::fs::read_to_string("/proc/self/status").ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("VmRSS:"))
            .and_then(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok()))
        .unwrap_or(0) / 1024
}

fn main() {
    let caminho = std::env::args().nth(1).expect("informe o .xlsx");
    if let Some(n) = std::env::var("NUCLEOS").ok().and_then(|n| n.parse().ok()) {
        feathersheet_core::paralelismo::definir(n);
    }
    let t = Instant::now();
    let (mut w, ativa) = ler_xlsx_arquivo(std::path::Path::new(&caminho)).expect("abrir");
    println!("[abrir] {:.1} s · {} MB · aba ativa {ativa}", t.elapsed().as_secs_f64(), rss_mb());
    for linha in feathersheet_core::diario::recolher() {
        println!("  {linha}");
    }
    for (i, nome) in w.nomes().iter().enumerate() {
        let s = &w.aba(i).unwrap().sheet;
        let u = s.used_rect();
        println!("  aba {i} {nome:?}: até linha {} coluna {} · {} fórmulas compactas · {} por célula",
            u.r1 + 1, u.c1 + 1, s.formulas.size(), s.cells.len());
    }
    if std::env::var_os("SO_ABRIR").is_some() { return; }
    let dados = w.indice_por_nome("Dados").expect("aba Dados");
    let resumo = w.indice_por_nome("Resumo").expect("aba Resumo");
    let pedidos: &[(&str, usize, u32, u32)] = &[
        ("Dados!G2 (E*F)", dados, 1, 6),
        ("Dados!L2 (XLOOKUP)", dados, 1, 11),
        ("Dados!M2 (IF aninhado)", dados, 1, 12),
        ("Dados!K1000001 (corrente de 1 M)", dados, 1_000_000, 10),
        ("Resumo!B2 (SUMIFS 1 M)", resumo, 1, 1),
        ("Resumo!N2 (SUM de 12 SUMIFS)", resumo, 1, 13),
    ];
    for (nome, aba, r, c) in pedidos {
        let t = Instant::now();
        let raw = w.aba(*aba).unwrap().sheet.get_raw(*r, *c);
        let v = w.valor(*aba, *r, *c);
        println!("[calc] {nome}: {raw} = {v:?} · {:.2} s · {} MB",
            t.elapsed().as_secs_f64(), rss_mb());
    }
    // O que a tela pede ao mostrar a aba Resumo: todas as células dela.
    let u = w.aba(resumo).unwrap().sheet.used_rect();
    let t = Instant::now();
    let mut contas = 0;
    for r in u.r0..=u.r1 {
        for c in u.c0..=u.c1 {
            let tc = Instant::now();
            let _ = w.valor(resumo, r, c);
            contas += 1;
            if tc.elapsed().as_secs_f64() > 0.15 {
                println!("  [lenta] Resumo L{}C{} {} · {:.2} s", r + 1, c + 1,
                    w.aba(resumo).unwrap().sheet.get_raw(r, c), tc.elapsed().as_secs_f64());
            }
        }
    }
    println!("[calc] Resumo inteira ({contas} células) · {:.2} s · {} MB",
        t.elapsed().as_secs_f64(), rss_mb());
    let dados_idx = dados;
    println!("[memória] Dados: {}", w.aba(dados).unwrap().sheet.contagem_memoria());
    w.soltar_faixas();
    println!("[memória] depois de soltar as faixas lidas: {} MB", rss_mb());
    let t = Instant::now();
    let mut total = 0;
    loop {
        let m = w.consolidar_calculos(1_000_000);
        if m == 0 { break; }
        total += m;
    }
    println!("[memória] {total} resultados para as colunas em {:.2} s: {} MB", t.elapsed().as_secs_f64(), rss_mb());
    println!("[memória] Dados: {}", w.aba(dados_idx).unwrap().sheet.contagem_memoria());
    let mut por_coluna = std::collections::BTreeMap::<u32, (usize, String)>::new();
    let s = &w.aba(dados_idx).unwrap().sheet;
    for (k, texto) in s.textos.iter() {
        let e = por_coluna.entry(feathersheet_core::celulas::refs::key_col(*k)).or_default();
        e.0 += 1;
        if e.1.is_empty() { e.1 = format!("{texto:?} (lido: {:?})", s.get_value(feathersheet_core::celulas::refs::key_row(*k), feathersheet_core::celulas::refs::key_col(*k))); }
    }
    println!("[textos] {por_coluna:?}");
}
