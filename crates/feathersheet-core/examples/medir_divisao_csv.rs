//! Quanto o "Texto para Colunas" gasta, etapa por etapa, num CSV real.
//!
//! Imita o caminho do assistente (`assistente_divisao.rs`, fases Validando e
//! Gravando) sem a interface: cada linha do CSV vira texto na coluna A da
//! aba 0; depois a conversão roda em lotes de `tamanho_lote` linhas, com as
//! mesmas opções do assistente (vírgula, consecutivos como um, aspas `"`).
//!
//! Etapas medidas:
//! - validação: leitura da célula + `contar_colunas_divisao` (por lote);
//! - gravação, por lote: leitura da célula + `dividir_com_opcoes`,
//!   `aplicar_divisao` e `compactar_lote_divisao` (dentro de
//!   `editar_lote_sem_notificar`, como no assistente), mais o
//!   `abrir_transacao`/`fechar_transacao` do destino.
//!
//! Não altera o motor: só mede com `Instant` e lê `/proc/self/status`.
//!
//! ```bash
//! nice -n 19 cargo run --release -j 2 --example medir_divisao_csv -- arquivo.csv 512
//! ```

use std::borrow::Cow;
use std::io::{BufRead, BufReader};
use std::time::{Duration, Instant};

use feathersheet_core::operacoes::ops::{self, OpcoesDivisao};
use feathersheet_core::planilha::workbook::Workbook;

/// Lê um campo de `/proc/self/status` em MB (`VmRSS:`, `VmHWM:`…).
fn status_mb(chave: &str) -> f64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for linha in status.lines() {
        if let Some(resto) = linha.strip_prefix(chave) {
            let kb: f64 = resto.trim().trim_end_matches(" kB").trim().parse().unwrap_or(0.0);
            return kb / 1024.0;
        }
    }
    0.0
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(caminho) = args.get(1) else {
        eprintln!("uso: medir_divisao_csv <arquivo.csv> [tamanho_lote]");
        return;
    };
    let lote: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(512).max(1);
    let opcoes = OpcoesDivisao {
        delimitadores: vec![",".into()],
        consecutivos_como_um: true,
        qualificador_texto: Some('"'),
    };

    // ---------------------------------------------------------- carga
    let arquivo = std::fs::File::open(caminho).expect("abrir o CSV");
    let mut wb = Workbook::new();
    let t0 = Instant::now();
    let mut n: usize = 0;
    {
        let aba = wb.aba_mut(0).expect("aba 0");
        for linha in BufReader::with_capacity(1 << 20, arquivo).lines() {
            let linha = linha.expect("ler linha");
            aba.sheet.set_cell_raw(n as u32, 0, linha, None);
            n += 1;
        }
    }
    let carga = t0.elapsed();
    println!(
        "carga: {n} linhas em {:.2} s · RSS {:.0} MB · pico {:.0} MB · lote {lote}",
        carga.as_secs_f64(),
        status_mb("VmRSS:"),
        status_mb("VmHWM:"),
    );

    // ------------------------------------------------------ validação
    let mut val_leitura = Duration::ZERO;
    let mut val_contar = Duration::ZERO;
    let mut val_pior_lote = Duration::ZERO;
    let mut cols = 0usize;
    let mut com_divisao = false;
    let t_val = Instant::now();
    {
        let sheet = &wb.aba(0).expect("aba 0").sheet;
        let mut inicio = 0usize;
        while inicio < n {
            let fim = (inicio + lote).min(n);
            let tl = Instant::now();
            for r in inicio..fim {
                let t = Instant::now();
                let raw: Cow<'_, str> = match sheet.texto_cru_divisao(r as u32, 0) {
                    Some(v) => Cow::Borrowed(v),
                    None => Cow::Owned(sheet.get_raw(r as u32, 0)),
                };
                val_leitura += t.elapsed();
                if raw.is_empty() {
                    continue;
                }
                let t = Instant::now();
                let c = ops::contar_colunas_divisao(&raw, &opcoes);
                val_contar += t.elapsed();
                if c > 1 {
                    com_divisao = true;
                }
                cols = cols.max(c);
            }
            val_pior_lote = val_pior_lote.max(tl.elapsed());
            inicio = fim;
        }
    }
    let val_total = t_val.elapsed();
    println!(
        "validação: total {:.2} s (leitura {:.2} s, contar {:.2} s) · pior lote {:.1} ms · até {cols} colunas · dividiu={com_divisao} · RSS {:.0} MB · pico {:.0} MB",
        val_total.as_secs_f64(),
        val_leitura.as_secs_f64(),
        val_contar.as_secs_f64(),
        ms(val_pior_lote),
        status_mb("VmRSS:"),
        status_mb("VmHWM:"),
    );

    // -------------------------------------------------------- gravação
    let destino = wb.criar_aba("Dividida");
    wb.editar_lote_sem_notificar(destino, |s| s.abrir_transacao());

    let mut w_ler = Duration::ZERO;
    let mut w_dividir = Duration::ZERO;
    let mut w_aplicar = Duration::ZERO;
    let mut w_compactar = Duration::ZERO;
    let mut pior_ler = Duration::ZERO;
    let mut pior_dividir = Duration::ZERO;
    let mut pior_aplicar = Duration::ZERO;
    let mut pior_compactar = Duration::ZERO;
    let mut pior_lote = Duration::ZERO;
    let mut lotes = 0usize;
    let mut idx_pior_aplicar = 0usize;
    let mut rss_pior_aplicar = 0.0f64;
    let t_w = Instant::now();
    let mut inicio = 0usize;
    while inicio < n {
        let fim = (inicio + lote).min(n);
        let tl = Instant::now();
        let mut linhas: Vec<(u32, Vec<String>)> = Vec::with_capacity(fim - inicio);
        let mut t_ler = Duration::ZERO;
        let mut t_div = Duration::ZERO;
        {
            // Leitura da origem (imutável), como o ramo "nova aba" do assistente.
            let sheet = &wb.aba(0).expect("aba 0").sheet;
            for r in inicio..fim {
                let t = Instant::now();
                let raw: Cow<'_, str> = match sheet.texto_cru_divisao(r as u32, 0) {
                    Some(v) => Cow::Borrowed(v),
                    None => Cow::Owned(sheet.get_raw(r as u32, 0)),
                };
                t_ler += t.elapsed();
                if raw.is_empty() {
                    continue;
                }
                let t = Instant::now();
                let partes = ops::dividir_com_opcoes(&raw, &opcoes);
                t_div += t.elapsed();
                linhas.push((r as u32, partes));
            }
        }
        let mut t_aplicar = Duration::ZERO;
        let mut t_compactar = Duration::ZERO;
        if !linhas.is_empty() {
            wb.editar_lote_sem_notificar(destino, |s| {
                let a = Instant::now();
                ops::aplicar_divisao(s, 0, &linhas, None);
                t_aplicar = a.elapsed();
                let c = Instant::now();
                s.compactar_lote_divisao();
                t_compactar = c.elapsed();
            });
        }
        w_ler += t_ler;
        w_dividir += t_div;
        w_aplicar += t_aplicar;
        w_compactar += t_compactar;
        pior_ler = pior_ler.max(t_ler);
        pior_dividir = pior_dividir.max(t_div);
        if t_aplicar > pior_aplicar {
            idx_pior_aplicar = lotes;
            rss_pior_aplicar = status_mb("VmRSS:");
        }
        pior_aplicar = pior_aplicar.max(t_aplicar);
        pior_compactar = pior_compactar.max(t_compactar);
        pior_lote = pior_lote.max(tl.elapsed());
        lotes += 1;
        inicio = fim;
    }
    let escrita = t_w.elapsed();
    let rss_antes_fechar = status_mb("VmRSS:");

    let t_f = Instant::now();
    wb.editar(destino, |s| s.fechar_transacao());
    let fechar = t_f.elapsed();

    println!(
        "gravação: {lotes} lotes · total {:.2} s (ler {:.2}, dividir {:.2}, aplicar {:.2}, compactar {:.2}) · fechar {:.3} s",
        escrita.as_secs_f64(),
        w_ler.as_secs_f64(),
        w_dividir.as_secs_f64(),
        w_aplicar.as_secs_f64(),
        w_compactar.as_secs_f64(),
        fechar.as_secs_f64(),
    );
    println!(
        "pior lote gravação: total {:.1} ms (ler {:.1}, dividir {:.1}, aplicar {:.1}, compactar {:.1})",
        ms(pior_lote),
        ms(pior_ler),
        ms(pior_dividir),
        ms(pior_aplicar),
        ms(pior_compactar),
    );
    println!(
        "pior aplicar no lote #{idx_pior_aplicar} (linhas {}..) · RSS nesse momento {rss_pior_aplicar:.0} MB",
        idx_pior_aplicar * lote,
    );
    println!(
        "após dividir: RSS {:.0} MB (antes do fechar: {:.0} MB) · pico {:.0} MB",
        status_mb("VmRSS:"),
        rss_antes_fechar,
        status_mb("VmHWM:"),
    );
    println!(
        "RESULT lote={lote} linhas={n} cols={cols} val_total_s={:.2} val_pior_ms={:.1} grav_total_s={:.2} grav_pior_ms={:.1} ler_s={:.2} dividir_s={:.2} aplicar_s={:.2} compactar_s={:.2} pior_ler_ms={:.1} pior_dividir_ms={:.1} pior_aplicar_ms={:.1} pior_compactar_ms={:.1} rss_carga_mb={:.0} pico_carga_mb={:.0} rss_final_mb={:.0} pico_final_mb={:.0}",
        val_total.as_secs_f64(),
        ms(val_pior_lote),
        escrita.as_secs_f64(),
        ms(pior_lote),
        w_ler.as_secs_f64(),
        w_dividir.as_secs_f64(),
        w_aplicar.as_secs_f64(),
        w_compactar.as_secs_f64(),
        ms(pior_ler),
        ms(pior_dividir),
        ms(pior_aplicar),
        ms(pior_compactar),
        status_mb("VmRSS:"),
        status_mb("VmHWM:"),
        status_mb("VmRSS:"),
        status_mb("VmHWM:"),
    );
}
