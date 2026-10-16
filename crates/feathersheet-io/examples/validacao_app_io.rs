//! Smoke APP-011: importação CSV/TSV comum sem substituir codecs XLSX/FSHZ.
use feathersheet_core::planilha::workbook::Workbook;
use feathersheet_io::{csv_workbook, da_workbook, fshz_read, para_workbook, xlsx_read, xlsx_write};

fn main() -> Result<(), String> {
    let csv = b"Produto;Valor\nCafe;1.234,56\n";
    let pasta = csv_workbook::importar_csv_tsv(csv.to_vec(), false)?;
    assert_eq!(pasta.aba(0).unwrap().sheet.get_raw(1, 0), "Cafe");
    assert_eq!(pasta.aba(0).unwrap().sheet.get_raw(1, 1), "1.234,56");

    let tsv = b"1\t=A1*2\n";
    let colunar = csv_workbook::importar_csv_tsv(tsv.to_vec(), true)?;
    assert_eq!(colunar.aba(0).unwrap().sheet.get_raw(0, 1), "=A1*2");

    let mut nativa = Workbook::new();
    nativa.escrever_celula(0, 0, 0, "42".into());
    nativa.escrever_celula(0, 0, 1, "=A1*2".into());
    let fshz = da_workbook::pasta_gravar_bytes(&nativa, 0)?;
    let reaberta = para_workbook::para_workbook(&fshz_read::ler_fshz(&fshz)?);
    assert_eq!(reaberta.aba(0).unwrap().sheet.get_raw(0, 1), "=A1*2");

    let xlsx = xlsx_write::gerar_xlsx(&nativa)?;
    let json = xlsx_read::ler_xlsx_bytes(xlsx)?;
    assert_eq!(json["sheets"][0]["name"], "Planilha1");

    println!("APP-011 OK: CSV/TSV comum, formula colunar e codecs XLSX/FSHZ nativos.");
    Ok(())
}
