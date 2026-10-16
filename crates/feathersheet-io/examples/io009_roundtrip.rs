//! IO-009: FSHZ GZip/FSH2/Arrow -> ler -> reconstruir -> regravar -> reler.
//! cargo run --manifest-path feathersheet/crates/feathersheet-io/Cargo.toml --example io009_roundtrip -- /tmp/io009.fshz
use feathersheet_io::{
    fshz_read::{ler_fshz, PastaLida},
    fshz_write::{gerar_fshz2, PastaBruta},
};
use serde_json::{json, Value as Json};

fn corpus() -> Result<PastaBruta, String> {
    let mut bitmap = vec![255u8; 33]; // 260 linhas, quatro bits finais.
    bitmap[32] = 0x0f;
    let dados = json!({
        "a": 0, "names": null,
        "sheets": [{
            "n": "Medições ⚡",
            "linhas": 260,
            "avulsas": [
                {"k": 0, "v": "café, ação, 日本語", "estilo": {"bold": true}},
                {"k": 1, "v": "=SOMA(10;20)", "estilo": null}
            ],
            "estilos_coluna": [[2, {"numFmt": "0.00"}]],
            "g": [{"type": "column", "title": "Energia mensal"}],
            "colunas_num": [{
                "col": 2,
                "values": (0..260).map(|v| v as f64 * 2.5).collect::<Vec<_>>(),
                "bitmap": bitmap
            }]
        }]
    });
    serde_json::from_value(dados).map_err(|e| e.to_string())
}

/// Reconstituição explícita dos dados lidos; não depende de bytes de compressão.
/// Somente os campos do corpus são usados, evitando testes triviais do mesmo buffer.
fn reconstituir(lida: &PastaLida) -> Result<PastaBruta, String> {
    let abas: Vec<Json> = lida.sheets.iter().map(|aba| {
        json!({
            "n": &aba.n,
            "tc": &aba.tc,
            "g": &aba.g,
            "pr": &aba.pr, "r": &aba.r, "tb": &aba.tb,
            "oq": &aba.oq, "dn": &aba.dn, "mc": &aba.mc,
            "im": &aba.im, "pd": &aba.pd, "va": &aba.va, "bt": &aba.bt,
            "avulsas": &aba.avulsas,
            "estilos_literais": &aba.estilos_diretos,
            "estilos_coluna": &aba.estilos_coluna,
            "estilos_linha": &aba.estilos_linha,
            "estilo_aba": &aba.estilo_aba,
            "linhas": aba.colunas_num.first().map(|c| c.values.len()).unwrap_or(0),
            "colunas_num": &aba.colunas_num,
            "colunas_txt": &aba.colunas_txt,
            "colunas_formula": &aba.colunas_formula,
            "padroes_formula": &aba.padroes_formula
        })
    }).collect();
    serde_json::from_value(json!({"a": lida.a, "names": &lida.names, "sheets": abas}))
        .map_err(|e| e.to_string())
}

fn executar(destino: &std::path::Path) -> Result<(), String> {
    let bytes = gerar_fshz2(&corpus()?)?;
    if !bytes.starts_with(b"FSH2") || !bytes[4..].starts_with(&[0x1f, 0x8b]) {
        return Err("FSHZ requer assinatura FSH2 seguida de GZip".into());
    }
    let primeira = ler_fshz(&bytes)?;
    if primeira.v != 2 || primeira.sheets.len() != 1 {
        return Err("FSH2 ou quantidade de abas incorreta".into());
    }
    let aba = &primeira.sheets[0];
    if aba.n != "Medições ⚡"
        || aba.avulsas.iter().map(|c| c.v.as_str()).collect::<Vec<_>>()
            != ["café, ação, 日本語", "=SOMA(10;20)"]
        || aba.colunas_num.len() != 1
        || aba.colunas_num[0].values.len() != 260
        || aba.colunas_num[0].values[259] != 647.5
    {
        return Err("FSH2 perdeu literal Unicode, fórmula ou número Arrow".into());
    }
    let novamente = gerar_fshz2(&reconstituir(&primeira)?)?;
    let segunda = ler_fshz(&novamente)?;
    if serde_json::to_value(&primeira).map_err(|e| e.to_string())?
        != serde_json::to_value(&segunda).map_err(|e| e.to_string())?
    {
        return Err("FSH2 mudou após regravar o objeto lido".into());
    }
    if ler_fshz(b"dados invalidos").is_ok() {
        return Err("FSH2 aceitou bytes inválidos".into());
    }
    std::fs::write(destino, &novamente).map_err(|e| e.to_string())?;
    eprintln!("IO-009 fshz: {} bytes, {} linhas, ida-volta semantica OK", novamente.len(), aba.colunas_num[0].values.len());
    Ok(())
}

fn main() {
    let destino = std::env::args().nth(1).unwrap_or_else(|| "/tmp/io009.fshz".into());
    if let Err(e) = executar(std::path::Path::new(&destino)) {
        eprintln!("ERRO IO-009 fshz: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn io009_fshz_arquivo_e_roundtrip_real() {
        let arquivo = std::env::temp_dir().join(format!("io009-{}-sheet.fshz", std::process::id()));
        executar(&arquivo).unwrap();
        assert!(ler_fshz(&std::fs::read(&arquivo).unwrap()).is_ok());
        std::fs::remove_file(arquivo).unwrap();
    }
}
