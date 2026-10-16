//! Quanto custa, no motor em Rust, o que travava o WebKit.
//!
//! O usuário relatou em 22/09/2026: "selecionei uma coluna e fiz ctrl -,
//! travou a planilha, estouro de memória com webkit", e junto a régua —
//! "apagar 10 milhões de células não deveria ser um problema".
//!
//! Este exemplo monta a mesma planilha que ele tinha, do lado Rust, e mede as
//! três coisas que importam: encher, excluir uma coluna e montar o quadro da
//! tela. Não é teste (não entra no `cargo test`, que precisa continuar
//! rodando em segundos) — é para rodar à mão quando se quiser o número:
//!
//! ```bash
//! cargo run --release --example medir_exclusao
//! ```

use std::time::Instant;

use feathersheet_core::planilha::frame::build_frame;
use feathersheet_core::planilha::workbook::Workbook;

/// O pico de memória do processo, em MB, lido do `/proc` — o mesmo lugar de
/// onde o `memoria_arvore.rs` tira o número que aparece no aplicativo.
fn pico_mb() -> f64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for linha in status.lines() {
        if let Some(resto) = linha.strip_prefix("VmHWM:") {
            let kb: f64 = resto
                .trim()
                .trim_end_matches(" kB")
                .trim()
                .parse()
                .unwrap_or(0.0);
            return kb / 1024.0;
        }
    }
    0.0
}

fn main() {
    const LINHAS: u32 = 2_000_000;
    const COLUNAS: u32 = 5;

    let mut pasta = Workbook::new();

    let t0 = Instant::now();
    {
        let aba = pasta.aba_mut(0).unwrap();
        for r in 0..LINHAS {
            for c in 0..COLUNAS {
                // Uma coluna de texto e quatro de número, como a planilha do
                // usuário. O `199,9` é de propósito: é o literal pt-BR que
                // precisa virar número.
                let texto = if c == 1 {
                    format!("registro {r}")
                } else {
                    format!("{},{}", r % 10_000, r % 10)
                };
                aba.sheet.set_cell_raw(r, c, texto, None);
            }
        }
    }
    let encher = t0.elapsed();
    let depois_de_encher = pico_mb();

    println!(
        "encher {} células: {:.1} s · pico {:.0} MB",
        LINHAS as u64 * COLUNAS as u64,
        encher.as_secs_f64(),
        depois_de_encher,
    );

    // A operação que travava: excluir a coluna D.
    let t1 = Instant::now();
    let mudanca = pasta.excluir_colunas(0, 3, 1).unwrap();
    let excluir = t1.elapsed();
    println!(
        "excluir 1 coluna: {:.3} ms · versão agora {} · pico {:.0} MB",
        excluir.as_secs_f64() * 1000.0,
        mudanca.versao,
        pico_mb(),
    );

    // E o caminho quente: o quadro de uma tela cheia, no meio da planilha.
    let aba = pasta.aba(0).unwrap();
    let t2 = Instant::now();
    let mut bytes = Vec::new();
    for _ in 0..100 {
        bytes = build_frame(&aba.sheet, aba.versao, 900_000, 0, 900_040, 3, None);
    }
    let quadro = t2.elapsed() / 100;
    println!(
        "quadro de 41x4 células: {:.3} ms · {} bytes",
        quadro.as_secs_f64() * 1000.0,
        bytes.len(),
    );

    // O mesmo quadro quando o TypeScript já tem a versão: a resposta curta.
    let t3 = Instant::now();
    let mut curto = Vec::new();
    for _ in 0..100 {
        curto = build_frame(&aba.sheet, aba.versao, 900_000, 0, 900_040, 3, Some(aba.versao));
    }
    println!(
        "quadro sem mudança: {:.4} ms · {} bytes",
        (t3.elapsed() / 100).as_secs_f64() * 1000.0,
        curto.len(),
    );
}
