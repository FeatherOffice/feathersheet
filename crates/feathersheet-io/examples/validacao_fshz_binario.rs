//! Diagnóstico FSH3. Nunca sobrescreve um arquivo existente.
//! `... --example validacao_fshz_binario -- converter origem.fshz destino.fshz`
//! `... --example validacao_fshz_binario -- ler arquivo.fshz`
//! `... --example validacao_fshz_binario -- fixture destino.fshz`
use std::{fs::OpenOptions, hash::Hasher, io::BufReader, path::Path, time::Instant};

struct HashBinario(std::collections::hash_map::DefaultHasher);
impl std::io::Write for HashBinario {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.write(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn executar() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let inicio = Instant::now();
    match args.as_slice() {
        [modo, estrategia, origem] if modo == "csv" => {
            let pasta = if estrategia == "fluxo" {
                feathersheet_io::csv_workbook::importar_csv_tsv_arquivo(
                    Path::new(origem),
                    false,
                    Some(','),
                )?
            } else if estrategia == "inteiro" {
                feathersheet_io::csv_workbook::importar_csv_tsv_com_separador(
                    std::fs::read(origem).map_err(|e| e.to_string())?,
                    false,
                    Some(','),
                )?
            } else {
                return Err("Use csv fluxo|inteiro caminho.csv".into());
            };
            let aba = &pasta.aba(0).unwrap().sheet;
            println!(
                "estrategia={estrategia} linhas={} colunas={} segundos={:.2}",
                aba.max_row + 1,
                aba.max_col + 1,
                inicio.elapsed().as_secs_f64()
            );
            // Amostras dos dois extremos detectam truncamento sem copiar a pasta.
            for r in [0, 1, aba.max_row / 2, aba.max_row] {
                let mut hash = std::collections::hash_map::DefaultHasher::new();
                for c in 0..=aba.max_col {
                    std::hash::Hash::hash(&aba.get_raw(r, c), &mut hash);
                }
                println!("linha={r} hash={:016x}", hash.finish());
            }
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                for linha in status
                    .lines()
                    .filter(|l| l.starts_with("VmRSS:") || l.starts_with("VmHWM:"))
                {
                    println!("{linha}");
                }
            }
        }
        [modo, origem, destino] if modo == "converter" => {
            let entrada = std::fs::File::open(origem).map_err(|e| e.to_string())?;
            let saida = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destino)
                .map_err(|e| e.to_string())?;
            if let Err(e) =
                feathersheet_io::fshz_read::converter_fshz2(BufReader::new(entrada), saida)
            {
                let _ = std::fs::remove_file(destino);
                return Err(e);
            }
            println!(
                "origem_bytes={} destino_bytes={} segundos={:.2}",
                std::fs::metadata(origem).map_err(|e| e.to_string())?.len(),
                std::fs::metadata(destino).map_err(|e| e.to_string())?.len(),
                inicio.elapsed().as_secs_f64()
            );
        }
        [modo, origem] if modo == "ler" => {
            let mut pasta = feathersheet_io::fshz_read::ler_fshz_arquivo(Path::new(origem))?;
            for aba in &mut pasta.sheets {
                aba.avulsas.sort_by_key(|c| c.k);
                println!(
                    "aba={} avulsas={} colunas_num={} colunas_txt={} formulas={}",
                    aba.n,
                    aba.avulsas.len(),
                    aba.colunas_num.len(),
                    aba.colunas_txt.len(),
                    aba.colunas_formula.len()
                );
            }
            let mut hash = HashBinario(std::collections::hash_map::DefaultHasher::new());
            rmp_serde::encode::write_named(&mut hash, &(pasta.a, &pasta.names, &pasta.sheets))
                .map_err(|e| e.to_string())?;
            println!("hash_semantico={:016x}", hash.0.finish());
            println!(
                "versao={} segundos={:.2}",
                pasta.v,
                inicio.elapsed().as_secs_f64()
            );
        }
        [modo, destino] if modo == "fixture" => {
            let mut wb = feathersheet_core::planilha::workbook::Workbook::new();
            wb.escrever_celula(0, 0, 0, "ação 🦜,;\"\nsegunda linha".into());
            wb.escrever_celula(0, 0, 1, "=SOMA(1;2)".into());
            for linha in 1..301 {
                wb.escrever_celula(0, linha, 0, format!("texto {} 🦜", linha % 7));
                wb.escrever_celula(0, linha, 1, linha.to_string());
            }
            let pasta = feathersheet_io::da_workbook::da_workbook(&wb, 0);
            let arquivo = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destino)
                .map_err(|e| e.to_string())?;
            feathersheet_io::fshz_write::gravar_fshz_para(arquivo, &pasta)?;
        }
        _ => return Err("Use converter origem destino, ler arquivo ou fixture destino".into()),
    }
    Ok(())
}

fn main() {
    if let Err(e) = executar() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
