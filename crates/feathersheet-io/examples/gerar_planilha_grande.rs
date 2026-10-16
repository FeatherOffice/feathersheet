//! Gera uma planilha grande de TEXTO para medir abrir/rolar/ordenar:
//! `linhas` × `colunas`, cada célula com 12 caracteres sorteados de um
//! conjunto de 1.000 textos por coluna (como dado real: nomes, códigos),
//! cabeçalho na linha 1. Monta em lotes de 50 mil linhas e grava `.fshz`.
//!
//! Uso (a partir de `feathersheet/crates/feathersheet-io`):
//!   systemd-run --user --scope -p MemoryMax=14G cargo run --release \
//!     --example gerar_planilha_grande -- <linhas> <colunas> <arquivo.fshz>

use std::time::Instant;

use feathersheet_core::celulas::refs::col_name;
use feathersheet_core::planilha::workbook::Workbook;

fn rss_mb(chave: &str) -> f64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    status.lines().find_map(|l| l.strip_prefix(chave))
        .and_then(|r| r.trim().trim_end_matches(" kB").trim().parse::<f64>().ok())
        .map_or(0.0, |kb| kb / 1024.0)
}

/// Gerador pseudoaleatório simples e determinístico (xorshift).
struct Sorteio(u64);
impl Sorteio {
    fn proximo(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// O texto `k` (0..1000) da coluna `c`: 12 caracteres, letras e números.
fn texto(c: u32, k: u64) -> String {
    const ALFA: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ";
    let mut s = Sorteio(0x9E37_79B9_7F4A_7C15 ^ (u64::from(c) << 20) ^ k.wrapping_mul(0x2545_F491_4F6C_DD1D) | 1);
    let mut t = String::with_capacity(12);
    for i in 0..12 {
        let v = s.proximo();
        t.push(if i % 4 == 3 { char::from(b'0' + (v % 10) as u8) } else { char::from(ALFA[(v % ALFA.len() as u64) as usize]) });
    }
    t
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let linhas: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(200_000);
    let colunas: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(131);
    let arquivo = args.get(3).cloned().unwrap_or_else(|| std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("Documentos/FeatherSheet").join("planilha-grande.fshz").to_string_lossy().into_owned());
    let distintos = 1_000u64;
    let dicionario: Vec<Vec<String>> = (0..colunas).map(|c| (0..distintos).map(|k| texto(c, k)).collect()).collect();

    let inicio = Instant::now();
    let mut wb = Workbook::new();
    wb.renomear_aba(0, "Dados");
    let cab: Vec<String> = (0..colunas).map(|c| format!("Campo {}", col_name(c))).collect();
    wb.editar(0, |s| s.paste(0, 0, &[cab], false));
    let mut sorteio = Sorteio(0xC0FFEE);
    let lote = 50_000u32;
    let mut r = 1u32;
    while r <= linhas {
        let n = lote.min(linhas + 1 - r);
        let bloco: Vec<Vec<String>> = (0..n).map(|_| {
            (0..colunas as usize).map(|c| dicionario[c][(sorteio.proximo() % distintos) as usize].clone()).collect()
        }).collect();
        wb.editar(0, |s| s.paste(r, 0, &bloco, false));
        r += n;
        if (r - 1) % 1_000_000 < lote {
            eprintln!("{:>9} linhas · {:.0} s · RAM {:.0} MB", r - 1, inicio.elapsed().as_secs_f64(), rss_mb("VmRSS:"));
        }
    }
    let montar = inicio.elapsed().as_secs_f64();
    let ram = rss_mb("VmRSS:");
    let t = Instant::now();
    feathersheet_io::da_workbook::pasta_gravar_arquivo(&wb, 0, std::path::Path::new(&arquivo))?;
    let tamanho = std::fs::metadata(&arquivo).map(|m| m.len()).unwrap_or(0) as f64 / 1e6;
    println!(
        "{arquivo}: {linhas} linhas × {colunas} colunas ({:.0} milhões de células) · montar {montar:.0} s · RAM {ram:.0} MB (pico {:.0}) · gravar {:.0} s · arquivo {tamanho:.0} MB",
        f64::from(linhas) * f64::from(colunas) / 1e6, rss_mb("VmHWM:"), t.elapsed().as_secs_f64()
    );
    Ok(())
}
