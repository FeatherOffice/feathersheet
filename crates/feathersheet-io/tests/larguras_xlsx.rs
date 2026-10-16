//! Larguras de coluna e altura: o gravador declara o padrão e a ida e volta
//! devolve os mesmos pixels (largura gravada = px / 7; padrão do Excel 64 px = 9,140625).
use feathersheet_io::{xlsx_read::ler_xlsx_workbook, xlsx_write::gerar_xlsx};
use std::io::{Cursor, Read, Write};

fn xlsx() -> Vec<u8> {
    let mut saida = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut saida);
        let o = zip::write::SimpleFileOptions::default();
        let folha = r#"<worksheet><sheetFormatPr defaultColWidth="9.140625" defaultRowHeight="15"/><cols><col min="1" max="1" width="14.285714" customWidth="1"/><col min="3" max="3" width="30" customWidth="1"/></cols><sheetData><row r="2" ht="30" customHeight="1"><c r="A2"><v>1</v></c></row></sheetData></worksheet>"#;
        let partes = [
            ("xl/workbook.xml", r#"<workbook xmlns:r="x"><sheets><sheet name="A" sheetId="1" r:id="rId1"/></sheets></workbook>"#),
            ("xl/_rels/workbook.xml.rels", r#"<Relationships><Relationship Id="rId1" Target="worksheets/sheet1.xml"/></Relationships>"#),
            ("xl/worksheets/sheet1.xml", folha),
        ];
        for (n, c) in partes { zip.start_file(n, o).unwrap(); zip.write_all(c.as_bytes()).unwrap(); }
        zip.finish().unwrap();
    }
    saida.into_inner()
}

#[test]
fn larguras_ida_e_volta_em_px() {
    let (wb, _) = ler_xlsx_workbook(xlsx()).unwrap();
    let s = &wb.aba(0).unwrap().sheet;
    assert_eq!(s.largura_padrao, Some(64));
    assert!(s.col_widths.get(&0) == Some(&100) && s.col_widths.get(&2) == Some(&210), "{:?}", s.col_widths);
    let bytes = gerar_xlsx(&wb).unwrap();
    let mut z = zip::ZipArchive::new(Cursor::new(bytes.clone())).unwrap();
    let mut xml = String::new();
    z.by_name("xl/worksheets/sheet1.xml").unwrap().read_to_string(&mut xml).unwrap();
    assert!(xml.contains("defaultColWidth=\"9.14\"") && xml.contains("defaultRowHeight=\"15\""), "{xml}");
    let (v, _) = ler_xlsx_workbook(bytes).unwrap();
    let t = &v.aba(0).unwrap().sheet;
    assert_eq!(t.largura_padrao, Some(64));
    assert_eq!(t.col_widths, s.col_widths);
    assert_eq!(t.row_heights, s.row_heights);
}

#[test]
fn sem_largura_propria_declara_a_padrao_do_motor() {
    let (wb, _) = ler_xlsx_workbook(xlsx()).unwrap();
    let mut wb = wb;
    wb.aba_mut(0).unwrap().sheet.largura_padrao = None;
    let mut z = zip::ZipArchive::new(Cursor::new(gerar_xlsx(&wb).unwrap())).unwrap();
    let mut xml = String::new();
    z.by_name("xl/worksheets/sheet1.xml").unwrap().read_to_string(&mut xml).unwrap();
    assert!(xml.contains("defaultColWidth=\"17.43\""), "{xml}");
}
