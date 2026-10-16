//! Sublinhado duplo e bordas desenhadas (cor e estilo da linha) sobrevivem ao
//! `.fshz` e ao `.xlsx` (`<u val="double"/>`, `style="thick"`).
use feathersheet_core::celulas::estilo_parcial::Campo;
use feathersheet_core::celulas::refs::Rect;
use feathersheet_core::operacoes::desenho_borda::{desenhar_borda, CanetaBorda, ModoBorda};
use feathersheet_core::operacoes::sublinhado::{alternar_sublinhado, sublinhado_de, Sublinhado};
use feathersheet_core::planilha::workbook::Workbook;
use feathersheet_io::da_workbook::pasta_gravar_bytes;
use feathersheet_io::fshz_read::ler_fshz;
use feathersheet_io::para_workbook::para_workbook;
use feathersheet_io::xlsx_read::ler_xlsx_workbook;
use feathersheet_io::xlsx_write::gerar_xlsx;
use std::io::{Cursor, Read};

fn pasta() -> Workbook {
    let mut w = Workbook::new();
    w.escrever_celula(0, 0, 0, "duplo".into());
    w.escrever_celula(0, 0, 1, "simples".into());
    w.editar(0, |s| {
        alternar_sublinhado(s, &[Rect::cell(0, 0)], (0, 0), Sublinhado::Duplo);
        alternar_sublinhado(s, &[Rect::cell(0, 1)], (0, 1), Sublinhado::Simples);
        let caneta = CanetaBorda { cor: Some("#d93025".into()), estilo: Some("Grossa".into()) };
        desenhar_borda(s, &Rect::new(2, 1, 3, 2), ModoBorda::Contorno, &caneta).unwrap();
        let tracejada = CanetaBorda { cor: None, estilo: Some("Tracejada".into()) };
        desenhar_borda(s, &Rect::cell(5, 0), ModoBorda::Grade, &tracejada).unwrap();
    });
    w
}

fn conferir(w: &Workbook, origem: &str) {
    let s = &w.aba(0).unwrap().sheet;
    assert_eq!(sublinhado_de(s, 0, 0), Sublinhado::Duplo, "{origem}");
    assert_eq!(sublinhado_de(s, 0, 1), Sublinhado::Simples, "{origem}");
    let e = s.estilo_mesclado(2, 1).unwrap();
    assert_eq!(e.get("borderLineStyle"), Some(&Campo::Texto("Grossa".into())), "{origem}");
    let b = s.estilo_resolvido(2, 1).unwrap().borders.unwrap();
    assert!(b.top && b.left && !b.right && !b.bottom, "{origem}: {b:?}");
    assert_eq!(b.color.as_deref(), Some("#d93025"), "{origem}");
    let b = s.estilo_resolvido(3, 2).unwrap().borders.unwrap();
    assert!(b.bottom && b.right && !b.top, "{origem}: {b:?}");
    assert_eq!(s.estilo_mesclado(5, 0).unwrap().get("borderLineStyle"),
        Some(&Campo::Texto("Tracejada".into())), "{origem}");
}

#[test]
fn fshz_ida_e_volta() {
    let w = pasta();
    conferir(&w, "memória");
    let bytes = pasta_gravar_bytes(&w, 0).expect("gravar");
    conferir(&para_workbook(&ler_fshz(&bytes).expect("ler")), ".fshz");
}

#[test]
fn xlsx_ida_e_volta() {
    let bytes = gerar_xlsx(&pasta()).unwrap();
    let mut z = zip::ZipArchive::new(Cursor::new(bytes.clone())).unwrap();
    let mut xml = String::new();
    z.by_name("xl/styles.xml").unwrap().read_to_string(&mut xml).unwrap();
    assert!(xml.contains("<u val=\"double\"/>"), "{xml}");
    assert!(xml.contains("<u/>"), "{xml}");
    assert!(xml.contains("style=\"thick\""), "{xml}");
    assert!(xml.contains("style=\"dashed\""), "{xml}");
    let (w, _) = ler_xlsx_workbook(bytes).unwrap();
    conferir(&w, ".xlsx");
}
