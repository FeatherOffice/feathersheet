//! Quanto a planilha de dois milhões de linhas pesa **no Rust**.
//!
//! É a medida que interessa à migração: hoje esse mesmo arquivo carregado no
//! WebKit chega a ~3 GB, e é por isso que excluir uma coluna ou salvar levava
//! o aplicativo ao estouro. Aqui a mesma planilha entra na
//! [`Workbook`](feathersheet_core::planilha::workbook::Workbook) e o custo é medido no
//! processo inteiro.
//!
//! O arquivo é o `planilha_2m.csv` da raiz do repositório — o mesmo que o
//! usuário usou para relatar os travamentos. Ele tem três colunas de fórmula,
//! então isto também mede o caminho caro de verdade (analisar fórmula), não
//! só o de número.
//!
//! ```bash
//! cargo run --release --example medir_carga
//! ```

use std::time::Instant;

use feathersheet_core::planilha::workbook::Workbook;

fn pico_mb() -> f64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for linha in status.lines() {
        if let Some(resto) = linha.strip_prefix("VmHWM:") {
            let kb: f64 = resto.trim().trim_end_matches(" kB").trim().parse().unwrap_or(0.0);
            return kb / 1024.0;
        }
    }
    0.0
}

fn agora_mb() -> f64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for linha in status.lines() {
        if let Some(resto) = linha.strip_prefix("VmRSS:") {
            let kb: f64 = resto.trim().trim_end_matches(" kB").trim().parse().unwrap_or(0.0);
            return kb / 1024.0;
        }
    }
    0.0
}

/// Corta uma linha de CSV com `;`, respeitando aspas duplas — a mesma regra do
/// `io/delimited.ts`, o suficiente para este arquivo.
fn campos(linha: &str) -> Vec<String> {
    let mut saida = Vec::with_capacity(8);
    let mut atual = String::new();
    let mut dentro_de_aspas = false;
    let mut chars = linha.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if dentro_de_aspas && chars.peek() == Some(&'"') => {
                atual.push('"');
                chars.next();
            }
            '"' => dentro_de_aspas = !dentro_de_aspas,
            ';' if !dentro_de_aspas => saida.push(std::mem::take(&mut atual)),
            _ => atual.push(c),
        }
    }
    saida.push(atual);
    saida
}

fn main() {
    let caminho = concat!(env!("CARGO_MANIFEST_DIR"), "/../../planilha_2m.csv");
    let Ok(texto) = std::fs::read_to_string(caminho) else {
        eprintln!("planilha_2m.csv não encontrado — nada a medir");
        return;
    };
    println!("arquivo lido: {:.0} MB de texto · RSS {:.0} MB", texto.len() as f64 / 1e6, agora_mb());

    let mut pasta = Workbook::new();
    let t0 = Instant::now();
    let mut celulas = 0u64;
    {
        let aba = pasta.aba_mut(0).unwrap();
        for (r, linha) in texto.lines().enumerate() {
            if linha.is_empty() {
                continue;
            }
            for (c, campo) in campos(linha).into_iter().enumerate() {
                if campo.is_empty() {
                    continue;
                }
                aba.sheet.set_cell_raw(r as u32, c as u32, campo, None);
                celulas += 1;
            }
        }
    }
    let carga = t0.elapsed();
    // O texto do arquivo não faz mais parte da planilha: soltar antes de medir.
    drop(texto);

    println!(
        "{celulas} células carregadas: {:.1} s · RSS {:.0} MB · pico {:.0} MB",
        carga.as_secs_f64(),
        agora_mb(),
        pico_mb(),
    );

    // O caminho quente: um quadro de tela cheia no meio da planilha.
    let aba = pasta.aba(0).unwrap();
    let t1 = Instant::now();
    let mut bytes = Vec::new();
    for _ in 0..50 {
        bytes = feathersheet_core::planilha::frame::build_frame(&aba.sheet, aba.versao, 1_000_000, 0, 1_000_040, 5, None);
    }
    println!(
        "quadro de 41x6 (com fórmula calculada): {:.3} ms · {} bytes",
        (t1.elapsed() / 50).as_secs_f64() * 1000.0,
        bytes.len(),
    );

    // E a operação que travava o aplicativo.
    let t2 = Instant::now();
    pasta.excluir_colunas(0, 3, 1);
    println!(
        "excluir 1 coluna: {:.3} ms · RSS {:.0} MB · pico {:.0} MB",
        t2.elapsed().as_secs_f64() * 1000.0,
        agora_mb(),
        pico_mb(),
    );
}
