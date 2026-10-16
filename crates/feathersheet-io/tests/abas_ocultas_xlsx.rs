//! Aba oculta: lida do workbook.xml (caminho nativo) e gravada com `state`.
use feathersheet_io::{xlsx_read::ler_xlsx_workbook, xlsx_write::gerar_xlsx};
use std::io::{Cursor, Read, Write};

fn xlsx_com_estados() -> Vec<u8> {
    let mut saida = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut saida);
        let o = zip::write::SimpleFileOptions::default();
        let folha = r#"<worksheet><sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData></worksheet>"#;
        let partes = [
            ("xl/workbook.xml", r#"<workbook xmlns:r="x"><sheets><sheet name="Vis" sheetId="1" r:id="rId1"/><sheet name="Oc" sheetId="2" state="hidden" r:id="rId2"/><sheet name="MuitoOc" sheetId="3" state="veryHidden" r:id="rId3"/></sheets></workbook>"#),
            ("xl/_rels/workbook.xml.rels", r#"<Relationships><Relationship Id="rId1" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Target="worksheets/sheet2.xml"/><Relationship Id="rId3" Target="worksheets/sheet3.xml"/></Relationships>"#),
            ("xl/worksheets/sheet1.xml", folha), ("xl/worksheets/sheet2.xml", folha), ("xl/worksheets/sheet3.xml", folha),
        ];
        for (n, c) in partes { zip.start_file(n, o).unwrap(); zip.write_all(c.as_bytes()).unwrap(); }
        zip.finish().unwrap();
    }
    saida.into_inner()
}

fn workbook_xml(bytes: Vec<u8>) -> String {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut s = String::new();
    z.by_name("xl/workbook.xml").unwrap().read_to_string(&mut s).unwrap();
    s
}

#[test]
fn aba_oculta_ida_e_volta() {
    let (wb, _) = ler_xlsx_workbook(xlsx_com_estados()).unwrap();
    let o: Vec<bool> = (0..3).map(|i| wb.aba(i).unwrap().sheet.oculta).collect();
    assert_eq!(o, [false, true, true]);
    let xml = workbook_xml(gerar_xlsx(&wb).unwrap());
    assert!(xml.contains(r#"name="Vis" sheetId="1" r:id"#), "{xml}");
    assert!(xml.contains(r#"name="Oc" sheetId="2" state="hidden""#), "{xml}");
    assert!(xml.contains(r#"name="MuitoOc" sheetId="3" state="hidden""#), "{xml}");
    assert!(xml.contains(r#"activeTab="0""#), "{xml}");
    let (de_volta, _) = ler_xlsx_workbook(gerar_xlsx(&wb).unwrap()).unwrap();
    assert!(de_volta.aba(1).unwrap().sheet.oculta && !de_volta.aba(0).unwrap().sheet.oculta);
}

#[test]
fn ativa_nunca_oculta() {
    let (mut wb, _) = ler_xlsx_workbook(xlsx_com_estados()).unwrap();
    wb.aba_mut(0).unwrap().sheet.oculta = true;
    wb.aba_mut(1).unwrap().sheet.oculta = false;
    let xml = workbook_xml(gerar_xlsx(&wb).unwrap());
    assert!(xml.contains(r#"activeTab="1""#), "{xml}");
    // todas ocultas: a primeira volta a ser visível
    for i in 0..3 { wb.aba_mut(i).unwrap().sheet.oculta = true; }
    let xml = workbook_xml(gerar_xlsx(&wb).unwrap());
    assert!(xml.contains(r#"activeTab="0""#) && !xml.contains(r#"sheetId="1" state"#), "{xml}");
}
