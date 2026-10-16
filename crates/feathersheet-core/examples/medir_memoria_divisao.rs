//! Onde vai a memória RAM do "Texto para Colunas", variante por variante.
//!
//! Cada variante roda em um processo próprio (o pico e o RSS não se misturam):
//!
//! - `a`: divisão como o app faz hoje (`aplicar_divisao`, com transação e
//!   histórico). Depois mede: fechar, trim, `clear_history`, trim de novo.
//! - `b`: mesma divisão, SEM histórico: `set_cell_divisao` direto, fora de
//!   qualquer `abrir_transacao` (com `pendente` = None nada é empilhado).
//!   Diferença `a − b` = custo do histórico.
//! - `e`: piso — só o destino, sem a fonte. Lê o CSV linha a linha, divide e
//!   grava cada campo com `set_cell_raw` (caminho público, sem transação).
//!
//! Medições por variante: VmRSS e VmHWM (`/proc/self/status`) e memória
//! VIVA (`mallinfo2().uordblks`, bytes em uso pelo malloc). RSS − viva =
//! memória retida pelo alocador (fragmentação/arenas) que o `malloc_trim`
//! não devolveu. Também conta células de texto/número, textos distintos no
//! pool e bytes dos textos, para calcular bytes por célula.
//!
//! ```bash
//! systemd-run --user --scope -p MemoryMax=3G -p MemorySwapMax=0 nice -n 19 timeout 300 \
//!   target/release/examples/medir_memoria_divisao <csv> a|b|e [lote]
//! ```

use std::io::{BufRead, BufReader};

use feathersheet_core::celulas::pool::StringPool;
use feathersheet_core::celulas::values::Value;
use feathersheet_core::operacoes::ops::{self, OpcoesDivisao};
use feathersheet_core::planilha::data_table::Cru;
use feathersheet_core::planilha::sheet::Sheet;
use feathersheet_core::planilha::workbook::Workbook;

extern "C" {
    fn malloc_trim(pad: usize) -> i32;
    fn mallinfo2() -> Mallinfo2;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Mallinfo2 {
    arena: usize,
    ordblks: usize,
    smblks: usize,
    hblks: usize,
    hblkhd: usize,
    usmblks: usize,
    fsmblks: usize,
    uordblks: usize,
    fordblks: usize,
    keepcost: usize,
}

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

/// Bytes em uso pelo malloc agora: `uordblks` (heap pequeno) mais `hblkhd`
/// (blocos grandes, >= 128 KiB, que o glibc aloca por mmap — os blocos de
/// coluna de 65 536 linhas moram aí e NÃO aparecem só em `uordblks`).
fn viva_mb() -> f64 {
    let m = unsafe { mallinfo2() };
    (m.uordblks + m.hblkhd) as f64 / MB
}

fn trim() {
    unsafe { malloc_trim(0) };
}

/// Contagens estruturais do destino: células de texto/número, textos
/// distintos e bytes do dicionário.
struct Estrutura {
    textos: usize,
    numeros: usize,
    distintos: usize,
    bytes_textos: usize,
    bytes_textos_com_cada_ocorrencia: usize,
    estilos_por_celula: usize,
    textos_preservados: usize,
    bytes_preservados: usize,
    colunas: usize,
    celulas_com_formula: usize,
}

fn estrutura(sheet: &Sheet, pool: &StringPool) -> Estrutura {
    let mut textos = 0usize;
    let mut numeros = 0usize;
    let mut bytes_com_ocorrencia = 0usize;
    let mut visita = |_r: u32, _c: u32, cru: Cru| match cru {
        Cru::Numero(_) => numeros += 1,
        Cru::Texto(id) => {
            textos += 1;
            bytes_com_ocorrencia += pool.get(id).len();
        }
    };
    sheet.data.for_each_cru(&mut visita);
    let distintos = pool.size();
    let bytes_textos: usize = (0..distintos as i32).map(|id| pool.get(id).len()).sum();
    Estrutura {
        textos,
        numeros,
        distintos,
        bytes_textos,
        bytes_textos_com_cada_ocorrencia: bytes_com_ocorrencia,
        estilos_por_celula: sheet.styles.len(),
        textos_preservados: sheet.textos.len(),
        bytes_preservados: sheet.textos.values().map(|t| t.len()).sum(),
        colunas: sheet.data.colunas().len(),
        celulas_com_formula: sheet.cells.len(),
    }
}

/// Quais células têm o texto bruto guardado em `Sheet::textos` (o que o
/// valor não reproduz, como `1.0` ou datas ISO), por tipo de valor, com
/// exemplos.
fn classificar_textos(sheet: &Sheet, pool: &StringPool) {
    let (mut num, mut txt, mut bo, mut outros) = (0usize, 0usize, 0usize, 0usize);
    let mut ex_num: Vec<String> = Vec::new();
    let mut ex_txt: Vec<String> = Vec::new();
    for (&k, raw) in sheet.textos.iter() {
        let (r, c) = Sheet::key_to_coord(k);
        match sheet.data.get_cell_value(r, c, pool) {
            Value::Number(_) => {
                num += 1;
                if ex_num.len() < 6 { ex_num.push(raw.to_string()); }
            }
            Value::Text(_) => {
                txt += 1;
                if ex_txt.len() < 6 { ex_txt.push(raw.to_string()); }
            }
            Value::Bool(_) => bo += 1,
            _ => outros += 1,
        }
    }
    println!(
        "textos preservados por tipo: número {num} (ex.: {ex_num:?}) · texto {txt} (ex.: {ex_txt:?}) · lógico {bo} · outros {outros}"
    );
}

fn opcoes() -> OpcoesDivisao {
    OpcoesDivisao {
        delimitadores: vec![",".into()],
        consecutivos_como_um: true,
        qualificador_texto: Some('"'),
    }
}

fn imprimir_estrutura(rotulo: &str, e: &Estrutura, viva_antes: f64, viva_depois: f64) {
    let celulas = e.textos + e.numeros;
    let delta = (viva_depois - viva_antes) * MB;
    println!(
        "{rotulo}: células {celulas} (texto {}, número {}) · textos distintos {} · bytes distintos {:.1} MB · bytes de texto por ocorrência {:.1} MB · estilos {} · colunas {} · textos brutos guardados (textos) {} = {:.1} MB · cells {}",
        e.textos,
        e.numeros,
        e.distintos,
        e.bytes_textos as f64 / MB,
        e.bytes_textos_com_cada_ocorrencia as f64 / MB,
        e.estilos_por_celula,
        e.colunas,
        e.textos_preservados,
        e.bytes_preservados as f64 / MB,
        e.celulas_com_formula,
    );
    if celulas > 0 {
        println!(
            "{rotulo}: viva delta {:.1} MB → {:.1} bytes por célula (texto bruto médio por célula {:.1} B)",
            delta / MB,
            delta / celulas as f64,
            e.bytes_textos_com_cada_ocorrencia as f64 / celulas as f64,
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (Some(caminho), Some(modo)) = (args.get(1), args.get(2)) else {
        eprintln!("uso: medir_memoria_divisao <arquivo.csv> a|b|e [lote]");
        return;
    };
    let lote: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(512).max(1);
    let opc = opcoes();
    let mut wb = Workbook::new();
    let mut n_linhas = 0usize;
    let viva_base;

    // ------------------------------------------------- piso (e): sem fonte
    if modo == "e" || modo == "n" {
        let arquivo = std::fs::File::open(caminho).expect("abrir o CSV");
        let destino = wb.aba_mut(0).expect("aba 0");
        let s = &mut destino.sheet;
        for (r, linha) in BufReader::with_capacity(1 << 20, arquivo).lines().enumerate() {
            let linha = linha.expect("ler linha");
            let partes = ops::dividir_com_opcoes(&linha, &opc);
            for (i, p) in partes.into_iter().enumerate() {
                // "n": só números (sem texto nem lógico), para separar o custo
                // do pool de textos e das colunas de texto do das de número.
                if modo == "n" && !matches!(
                    feathersheet_core::celulas::input::parse_literal(p.trim()).value,
                    Value::Number(_)
                ) {
                    continue;
                }
                s.set_cell_raw(r as u32, i as u32, p.trim().to_string(), None);
            }
            n_linhas += 1;
        }
        trim();
        viva_base = viva_mb();
        println!(
            "{modo} (sem fonte): linhas {n_linhas} · RSS após trim {:.0} MB · viva {:.1} MB · pico {:.0} MB",
            status_mb("VmRSS:"),
            viva_base,
            status_mb("VmHWM:"),
        );
        let e = estrutura(s, &s.pool);
        imprimir_estrutura("e", &e, 0.0, viva_base);
        println!(
            "RESULT modo={modo} rss_trim_mb={:.0} viva_mb={:.1} pico_mb={:.0} celulas={} textos={} distintos={} bytes_distintos_mb={:.1}",
            status_mb("VmRSS:"),
            viva_base,
            status_mb("VmHWM:"),
            e.textos + e.numeros,
            e.textos,
            e.distintos,
            e.bytes_textos as f64 / MB,
        );
        return;
    }

    // --------------------------------------------- carga da fonte (a, b)
    {
        let arquivo = std::fs::File::open(caminho).expect("abrir o CSV");
        let aba = wb.aba_mut(0).expect("aba 0");
        for linha in BufReader::with_capacity(1 << 20, arquivo).lines() {
            let linha = linha.expect("ler linha");
            aba.sheet.set_cell_raw(n_linhas as u32, 0, linha, None);
            n_linhas += 1;
        }
    }
    trim();
    let viva_fonte = viva_mb();
    let fonte_estr = estrutura(&wb.aba(0).expect("aba 0").sheet, &wb.aba(0).expect("aba 0").sheet.pool);
    println!(
        "carga: {n_linhas} linhas · RSS após trim {:.0} MB · viva {viva_fonte:.1} MB · pico {:.0} MB · textos fonte {:.1} MB",
        status_mb("VmRSS:"),
        status_mb("VmHWM:"),
        fonte_estr.bytes_textos as f64 / MB,
    );

    // ----------------------------------------------- gravação (a, b)
    let destino = wb.criar_aba("Dividida");
    let com_historico = modo == "a";
    if com_historico {
        wb.editar_lote_sem_notificar(destino, |s| s.abrir_transacao());
    }
    let mut inicio = 0usize;
    while inicio < n_linhas {
        let fim = (inicio + lote).min(n_linhas);
        let mut linhas: Vec<(u32, Vec<String>)> = Vec::with_capacity(fim - inicio);
        {
            let sheet = &wb.aba(0).expect("aba 0").sheet;
            for r in inicio..fim {
                let raw = match sheet.texto_cru_divisao(r as u32, 0) {
                    Some(v) => v.to_string(),
                    None => sheet.get_raw(r as u32, 0),
                };
                if raw.is_empty() {
                    continue;
                }
                linhas.push((r as u32, ops::dividir_com_opcoes(&raw, &opc)));
            }
        }
        if !linhas.is_empty() {
            if com_historico {
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
        }
        inicio = fim;
    }
    let rss_sem_trim = status_mb("VmRSS:");
    let viva_grav = viva_mb();
    println!(
        "gravação ({modo}): antes de fechar · RSS {rss_sem_trim:.0} MB · viva {viva_grav:.1} MB · pico {:.0} MB · histórico profundidade {}",
        status_mb("VmHWM:"),
        wb.aba(destino).expect("destino").sheet.profundidade_do_historico(),
    );

    if com_historico {
        wb.editar(destino, |s| s.fechar_transacao());
        let rss_fechar = status_mb("VmRSS:");
        trim();
        let viva_fechado = viva_mb();
        println!(
            "C1 fechada, histórico retido: RSS sem trim {rss_fechar:.0} MB · RSS trim {:.0} MB · viva {viva_fechado:.1} MB · profundidade {}",
            status_mb("VmRSS:"),
            wb.aba(destino).expect("destino").sheet.profundidade_do_historico(),
        );
        wb.editar_lote_sem_notificar(destino, |s| s.clear_history());
        trim();
        println!(
            "C2 após clear_history: RSS trim {:.0} MB · viva {:.1} MB · profundidade {}",
            status_mb("VmRSS:"),
            viva_mb(),
            wb.aba(destino).expect("destino").sheet.profundidade_do_historico(),
        );
    } else {
        trim();
    }

    let viva_final = viva_mb();
    let dest_estr = estrutura(&wb.aba(destino).expect("destino").sheet, &wb.aba(destino).expect("destino").sheet.pool);
    imprimir_estrutura("destino", &dest_estr, viva_fonte, viva_final);
    classificar_textos(&wb.aba(destino).expect("destino").sheet, &wb.aba(destino).expect("destino").sheet.pool);
    let delta_rss = status_mb("VmRSS:");
    println!(
        "RESULT modo={modo} rss_trim_mb={delta_rss:.0} viva_mb={viva_final:.1} pico_mb={:.0} viva_fonte_mb={viva_fonte:.1} viva_delta_mb={:.1} celulas={} textos={} distintos={} bytes_distintos_mb={:.1} bytes_por_celula_viva={:.1} fonte_textos_mb={:.1} fonte_distintos={}",
        status_mb("VmHWM:"),
        viva_final - viva_fonte,
        dest_estr.textos + dest_estr.numeros,
        dest_estr.textos,
        dest_estr.distintos,
        dest_estr.bytes_textos as f64 / MB,
        (viva_final - viva_fonte) * MB / (dest_estr.textos + dest_estr.numeros).max(1) as f64,
        fonte_estr.bytes_textos as f64 / MB,
        fonte_estr.distintos,
    );
}
