//! Compara o caminho de importação de CSV com o do "Texto para Colunas".
//!
//! Cada modo roda em um processo próprio (pico e RSS não se misturam):
//!
//! - `gui`: `importar_csv_tsv_arquivo` (o que a GUI Iced usa ao abrir CSV):
//!   lê do disco em lotes e grava cada campo com `Sheet::set_cell_raw`.
//! - `ponte`: `Ponte::inserir_aba_de_csv` (o colar em massa com
//!   `Sheet::paste(record = false)`). O texto inteiro fica na memória, como
//!   nesse caminho de verdade.
//! - `div`: carrega o CSV numa coluna (uma célula = uma linha crua) e faz o
//!   Texto para Colunas em lotes de 512 linhas, como o assistente da GUI:
//!   transação externa, `aplicar_divisao` + `compactar_lote_divisao`, e
//!   `fechar_transacao` no fim. `consecutivos` = true (padrão do assistente).
//! - `divfiel`: igual a `div`, mas `consecutivos_como_um = false`, o que
//!   preserva campos vazios como a importação.
//! - `divsemhist`: `Sheet::set_cell_divisao` direto, SEM transação, então sem
//!   histórico. Isola o custo do histórico de desfazer.
//!
//! Medidas: tempo, VmHWM (pico RSS), células, `Sheet::textos` (texto cru
//! preservado), estilos, e textos distintos no pool.
//!
//! ```bash
//! systemd-run --user --scope -p MemoryMax=3G -p MemorySwapMax=0 nice -n 19 timeout 300 \
//!   target/release/examples/medir_importacao_divisao <csv> gui|ponte|div|divfiel|divsemhist
//! ```

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

use feathersheet_core::operacoes::ops::{self, OpcoesDivisao};
use feathersheet_core::planilha::data_table::Cru;
use feathersheet_core::planilha::sheet::Sheet;
use feathersheet_core::planilha::workbook::Workbook;
use feathersheet_ponte::Ponte;

const MB: f64 = 1024.0 * 1024.0;

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

/// (células, textos crus preservados, estilos, textos distintos no pool)
fn contagens(sheet: &Sheet) -> (usize, usize, usize, usize) {
    let mut celulas = 0usize;
    let mut visita = |_r: u32, _c: u32, _cru: Cru| celulas += 1;
    sheet.data.for_each_cru(&mut visita);
    (celulas + sheet.cells.len(), sheet.textos.len(), sheet.styles.len(), sheet.pool.size())
}

/// Duas primeiras linhas, colunas 0..8: o texto cru que a célula guarda (`get_raw`).
fn mostrar(sheet: &Sheet) {
    for linha in 0..2u32 {
        let celulas: Vec<String> = (0..8).map(|c| format!("{:?}", sheet.get_raw(linha, c))).collect();
        println!("LINHA{linha} {}", celulas.join(" | "));
    }
}

fn relatar(modo: &str, tempo_ms: f64, sheet: &Sheet) {
    let (celulas, textos, estilos, distintos) = contagens(sheet);
    println!(
        "RESULT modo={modo} tempo_ms={tempo_ms:.0} pico_mb={:.0} rss_mb={:.0} celulas={celulas} textos_crus={textos} estilos={estilos} distintos={distintos}",
        status_mb("VmHWM:"),
        status_mb("VmRSS:"),
    );
}

fn opcoes(consecutivos: bool) -> OpcoesDivisao {
    OpcoesDivisao {
        delimitadores: vec![",".into()],
        consecutivos_como_um: consecutivos,
        qualificador_texto: Some('"'),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (Some(caminho), Some(modo)) = (args.get(1), args.get(2)) else {
        eprintln!("uso: medir_importacao_divisao <csv> gui|ponte|div|divfiel|divsemhist");
        return;
    };
    let inicio = Instant::now();
    match modo.as_str() {
        "gui" => {
            let wb = feathersheet_io::csv_workbook::importar_csv_tsv_arquivo(Path::new(caminho), false, None)
                .expect("importar");
            let ms = inicio.elapsed().as_secs_f64() * 1000.0;
            relatar(modo, ms, &wb.aba(0).expect("aba 0").sheet);
            if std::env::var("MOSTRAR").is_ok() {
                mostrar(&wb.aba(0).expect("aba 0").sheet);
            }
        }
        "ponte" => {
            let texto = std::fs::read_to_string(caminho).expect("ler CSV");
            println!("texto do arquivo na memória: {:.0} MB", texto.len() as f64 / MB);
            let mut ponte = Ponte::default();
            let inicio = Instant::now();
            ponte.inserir_aba_de_csv("CSV".into(), &texto).expect("inserir");
            let ms = inicio.elapsed().as_secs_f64() * 1000.0;
            let ultima = ponte.pasta.len() - 1;
            relatar(modo, ms, &ponte.pasta.aba(ultima).expect("aba").sheet);
            if std::env::var("MOSTRAR").is_ok() {
                mostrar(&ponte.pasta.aba(ultima).expect("aba").sheet);
            }
        }
        "div" | "divfiel" | "divsemhist" | "so_dividir" => {
            let consecutivos = modo == "div";
            let com_historico = modo != "divsemhist";
            let so_dividir = modo == "so_dividir";
            let opc = opcoes(consecutivos);
            let mut wb = Workbook::new();
            let mut n_linhas = 0usize;
            {
                let arquivo = std::fs::File::open(caminho).expect("abrir CSV");
                let fonte = &mut wb.aba_mut(0).expect("aba 0").sheet;
                for linha in BufReader::with_capacity(1 << 20, arquivo).lines() {
                    let linha = linha.expect("ler linha");
                    fonte.set_cell_raw(n_linhas as u32, 0, linha, None);
                    n_linhas += 1;
                }
            }
            println!("fonte carregada: {n_linhas} linhas · pico {:.0} MB · viva/RSS {:.0} MB",
                status_mb("VmHWM:"), status_mb("VmRSS:"));
            let destino = wb.criar_aba("Dividida");
            let inicio = Instant::now();
            if com_historico {
                wb.editar_lote_sem_notificar(destino, |s| s.abrir_transacao());
            }
            let mut inicio_lote = 0usize;
            while inicio_lote < n_linhas {
                let fim = (inicio_lote + 512).min(n_linhas);
                let mut linhas: Vec<(u32, Vec<String>)> = Vec::with_capacity(fim - inicio_lote);
                {
                    let fonte = &wb.aba(0).expect("aba 0").sheet;
                    for r in inicio_lote..fim {
                        let raw: String = match fonte.texto_cru_divisao(r as u32, 0) {
                            Some(v) => v.to_string(),
                            None => fonte.get_raw(r as u32, 0),
                        };
                        if raw.is_empty() {
                            continue;
                        }
                        linhas.push((r as u32, ops::dividir_com_opcoes(&raw, &opc)));
                    }
                }
                if so_dividir {
                    // Só o custo de partir o texto, sem gravar nada.
                    std::hint::black_box(&linhas);
                } else if com_historico {
                    wb.editar_lote_sem_notificar(destino, |s| {
                        ops::aplicar_divisao(s, 0, &linhas, None);
                        s.compactar_lote_divisao();
                    });
                } else {
                    wb.editar_lote_sem_notificar(destino, |s| {
                        for (row, partes) in &linhas {
                            for (i, parte) in partes.iter().enumerate() {
                                s.set_cell_divisao(*row, i as u32, parte.trim());
                            }
                        }
                    });
                }
                inicio_lote = fim;
            }
            let fechar = Instant::now();
            let ms_sem_fechar = inicio.elapsed().as_secs_f64() * 1000.0;
            println!("tempo até o fim do laço (sem fechar): {ms_sem_fechar:.0} ms");
            if com_historico {
                wb.editar_lote_sem_notificar(destino, |s| s.fechar_transacao());
            }
            let fechar_ms = fechar.elapsed().as_secs_f64() * 1000.0;
            let ms = inicio.elapsed().as_secs_f64() * 1000.0;
            println!("fechar_transacao: {fechar_ms:.1} ms");
            relatar(modo, ms, &wb.aba(destino).expect("aba destino").sheet);
            if std::env::var("MOSTRAR").is_ok() {
                mostrar(&wb.aba(destino).expect("aba destino").sheet);
            }
        }
        outro => eprintln!("modo desconhecido: {outro}"),
    }
}
