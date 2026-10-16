//! Exportação JSON da pasta nativa, compatível com o contrato `Pasta.toJSON()`.
//!
//! Diferente de `JSON.stringify(workbook.toJSON())` do frontend TS, as
//! células são escritas diretamente no arquivo, sem materializar uma cópia
//! da pasta em uma árvore JSON ou uma matriz densa de linhas.
use std::io::{self, BufWriter, Write};
use std::path::Path;

use feathersheet_core::celulas::refs::{MAX_COLS, MAX_ROWS};
use feathersheet_core::planilha::sheet::Sheet;
use feathersheet_core::planilha::workbook::Workbook;

fn json<W: Write>(destino: &mut W, dado: &impl serde::Serialize) -> io::Result<()> {
    serde_json::to_writer(destino, dado).map_err(io::Error::other)
}

fn celula<W: Write>(
    destino: &mut W, aba: &Sheet, linha: u32, coluna: u32, primeira: &mut bool,
) -> io::Result<()> {
    let raw = aba.get_raw(linha, coluna);
    let estilo = aba.get_style(linha, coluna);
    if raw.is_empty() && estilo.is_none() {
        return Ok(());
    }
    if !*primeira { destino.write_all(b",")?; }
    *primeira = false;
    // A chave numérica do TypeScript é linha * MAX_COLS + coluna.
    json(destino, &((linha as u64 * MAX_COLS as u64 + coluna as u64).to_string()))?;
    destino.write_all(b":{\"raw\":")?;
    json(destino, &raw)?;
    if let Some(estilo) = estilo {
        destino.write_all(b",\"style\":")?;
        json(destino, &crate::para_workbook::estilo_para_json(estilo))?;
    }
    destino.write_all(b"}")
}

fn escrever_celulas<W: Write>(destino: &mut W, aba: &Sheet) -> io::Result<()> {
    destino.write_all(b"\"cells\":{")?;
    let mut primeira = true;

    // Fórmulas avulsas e entradas que não cabem nos blocos colunares.
    for &chave in aba.cells.keys() {
        let (r, c) = Sheet::key_to_coord(chave);
        celula(destino, aba, r, c, &mut primeira)?;
    }

    // Dados colunares. A callback visita só células presentes; não percorre
    // o retângulo inteiro de 10 milhões de linhas x 16.384 colunas.
    let mut falha = None;
    aba.data.for_each_cru(&mut |r, c, _| {
        if falha.is_none() && !aba.cells.contains_key(&Sheet::cell_key(r, c)) {
            if let Err(erro) = celula(destino, aba, r, c, &mut primeira) {
                falha = Some(erro);
            }
        }
    });
    if let Some(erro) = falha.take() { return Err(erro); }

    // Fórmulas por padrão, também sem `entries()` (que alocaria um Vec
    // com o texto de cada uma das fórmulas, inclusive milhões de entradas).
    aba.formulas.for_each_in_range(0, MAX_ROWS - 1, 0, MAX_COLS - 1, |r, c| {
        if falha.is_none() && !aba.cells.contains_key(&Sheet::cell_key(r, c))
            && !aba.data.has_cell(r, c)
        {
            if let Err(erro) = celula(destino, aba, r, c, &mut primeira) {
                falha = Some(erro);
            }
        }
    });
    if let Some(erro) = falha.take() { return Err(erro); }

    // Uma célula pode possuir estilo sem conteúdo.
    for &chave in aba.styles.keys() {
        let (r, c) = Sheet::key_to_coord(chave);
        if !aba.cells.contains_key(&chave) && !aba.data.has_cell(r, c)
            && !aba.formulas.has(r, c)
        {
            celula(destino, aba, r, c, &mut primeira)?;
        }
    }
    destino.write_all(b"}")
}

fn dimensoes<W: Write>(
    destino: &mut W, nome: &[u8], dimensoes: &std::collections::HashMap<u32, u32>,
) -> io::Result<()> {
    destino.write_all(nome)?;
    destino.write_all(b":[")?;
    let mut primeira = true;
    for (&indice, &tamanho) in dimensoes {
        if !primeira { destino.write_all(b",")?; }
        primeira = false;
        json(destino, &(indice, tamanho))?;
    }
    destino.write_all(b"]")
}

fn metadado<W: Write>(
    destino: &mut W, nome: &str, bruto: &Option<String>,
) -> io::Result<()> {
    if let Some(bruto) = bruto {
        // Evita produzir JSON inválido se um metadado herdado estiver corrompido.
        if let Ok(valor) = serde_json::from_str::<serde_json::Value>(bruto) {
            destino.write_all(b",")?;
            json(destino, &nome)?;
            destino.write_all(b":")?;
            json(destino, &valor)?;
        }
    }
    Ok(())
}

/// Escreve o contrato JSON v2 do frontend TypeScript:
/// `{ version, active, sheets: [{name, cells, ...}], names }`.
/// As fórmulas saem como texto original; os estilos diretos e metadados são
/// preservados. Não materializa as células da pasta como `serde_json::Value`.
pub fn exportar_json<W: Write>(
    pasta: &Workbook, ativa: usize, destino: &mut W,
) -> io::Result<()> {
    destino.write_all(b"{\"version\":2,\"active\":")?;
    json(destino, &ativa)?;
    destino.write_all(b",\"sheets\":[")?;
    for i in 0..pasta.len() {
        if i > 0 { destino.write_all(b",")?; }
        let aba = &pasta.aba(i).ok_or_else(||
            io::Error::new(io::ErrorKind::NotFound, "Aba inexistente")
        )?.sheet;
        destino.write_all(b"{\"name\":")?;
        json(destino, &aba.name)?;
        if aba.oculta {
            destino.write_all(b",\"oculta\":true")?;
        }
        if let Some(cor) = &aba.tab_color {
            destino.write_all(b",\"tabColor\":")?;
            json(destino, cor)?;
        }
        destino.write_all(b",")?;
        escrever_celulas(destino, aba)?;
        destino.write_all(b",")?;
        dimensoes(destino, b"\"colWidths\"", &aba.col_widths)?;
        destino.write_all(b",")?;
        dimensoes(destino, b"\"rowHeights\"", &aba.row_heights)?;

        // Mapeamento do FSHZ para os mesmos nomes do `SheetData` do TS.
        for (nome, valor) in [
            ("charts", &aba.g), ("protecoes", &aba.pr), ("rules", &aba.r),
            ("tabelas", &aba.tb), ("consultas", &aba.oq), ("dinamicas", &aba.dn),
            ("mesclas", &aba.mc), ("images", &aba.im),
            ("validations", &aba.va), ("buttons", &aba.bt),
        ] {
            metadado(destino, nome, valor)?;
        }
        destino.write_all(b"}")?;
    }
    destino.write_all(b"],\"names\":")?;
    json(destino, &crate::para_workbook::nomes_para_json(pasta))?;
    destino.write_all(b"}")
}

/// Grava a cópia JSON incrementalmente e faz `rename` somente quando a
/// escrita terminar sem erro, preservando o destino original caso falhe.
pub fn gravar_json(pasta: &Workbook, ativa: usize, path: &Path) -> io::Result<()> {
    crate::arquivo::gravar_atomico_com(path, |arquivo| {
        let mut writer = BufWriter::with_capacity(64 * 1024, arquivo);
        exportar_json(pasta, ativa, &mut writer)?;
        writer.flush()
    })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn exportar_json_preserva_estado_de_aba_oculta() {
        let mut pasta = Workbook::new();
        pasta.criar_aba("Segunda");
        assert_eq!(pasta.ocultar_aba(0), Ok(1));
        let mut bytes = Vec::new();
        exportar_json(&pasta, 1, &mut bytes).unwrap();
        let documento: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(documento["sheets"][0]["oculta"], true);
        assert!(documento["sheets"][1].get("oculta").is_none());
    }

    #[test]
    fn exporta_abas_formulas_acentos_e_metadados_no_contrato_ts() {
        let mut pasta = Workbook::new();
        pasta.escrever_celula(0, 0, 0, "Olá \"mundo\"".into());
        pasta.escrever_celula(0, 1, 1, "=A1".into());
        let i = pasta.criar_aba("Relatório");
        pasta.escrever_celula(i, 0, 0, "123".into());
        pasta.aba_mut(0).unwrap().sheet.g = Some(r#"[{"id":"grafico_1"}]"#.into());
        pasta.aba_mut(0).unwrap().sheet.col_widths.insert(0, 123);

        let mut bytes = Vec::new();
        exportar_json(&pasta, 1, &mut bytes).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["version"], 2);
        assert_eq!(v["active"], 1);
        assert_eq!(v["sheets"].as_array().unwrap().len(), 2);
        assert_eq!(v["sheets"][0]["name"], "Planilha1");
        assert_eq!(v["sheets"][0]["cells"]["0"]["raw"], "Olá \"mundo\"");
        assert_eq!(v["sheets"][0]["cells"][(MAX_COLS as u64 + 1).to_string().as_str()]["raw"], "=A1");
        assert_eq!(v["sheets"][0]["charts"][0]["id"], "grafico_1");
        assert_eq!(v["sheets"][0]["colWidths"][0], serde_json::json!([0,123]));
        assert_eq!(v["sheets"][1]["name"], "Relatório");
        assert!(v["names"].is_array());
        assert_eq!(pasta.aba(0).unwrap().sheet.get_raw(1, 1), "=A1");
    }

    #[test]
    fn celula_distante_nao_expande_em_matriz_densa() {
        let mut pasta = Workbook::new();
        pasta.escrever_celula(0, 100_000, 2, "fim".into());
        let mut bytes = Vec::new();
        exportar_json(&pasta, 0, &mut bytes).unwrap();
        assert!(bytes.len() < 2_000, "JSON esparso não deve conter 100 mil linhas vazias");
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["sheets"][0]["cells"][(100_000_u64 * MAX_COLS as u64 + 2).to_string().as_str()]["raw"], "fim");
    }

    #[test]
    fn erros_de_escrita_sao_propagados() {
        struct Falha;
        impl Write for Falha {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("disco cheio"))
            }
            fn flush(&mut self) -> io::Result<()> { Ok(()) }
        }
        assert!(exportar_json(&Workbook::new(), 0, &mut Falha).is_err());
    }
}
