//! Comentários (anotações) da aba no `.xlsx`, no formato clássico que todo
//! Excel lê: `xl/commentsN.xml` com autor e texto, e o desenho VML
//! `xl/drawings/vmlDrawingN.vml` com o balão de cada um (posição e se fica
//! sempre à mostra). A aba aponta para o VML com `<legacyDrawing>`.
//!
//! Como o Excel, o texto começa com o nome do autor em negrito seguido de
//! dois-pontos e quebra de linha; a leitura tira esse prefixo de volta.

use std::fmt::Write as _;

use feathersheet_core::celulas::refs::cell_name;
use feathersheet_core::planilha::sheet::Sheet;

use crate::xlsx_write::escape_xml;

const NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const XML_DECL: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n";

pub(crate) const TIPO_COMENTARIOS: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.comments+xml";
pub(crate) const TIPO_VML: &str = "application/vnd.openxmlformats-officedocument.vmlDrawing";

/// `xl/commentsN.xml` e `xl/drawings/vmlDrawingN.vml` da aba (`n` começa
/// em 1). `None` quando a aba não tem comentário.
pub(crate) fn partes(sheet: &Sheet, n: usize) -> Option<(String, String)> {
    if sheet.comentarios.is_empty() { return None; }
    let mut autores: Vec<&str> = Vec::new();
    let mut lista = String::new();
    for (&(r, c), cm) in &sheet.comentarios {
        let id = autores.iter().position(|a| *a == cm.autor).unwrap_or_else(|| {
            autores.push(&cm.autor);
            autores.len() - 1
        });
        let _ = write!(lista, "<comment ref=\"{}\" authorId=\"{id}\"><text>", cell_name(r, c));
        if !cm.autor.is_empty() {
            let _ = write!(
                lista,
                "<r><rPr><b/><sz val=\"9\"/><color indexed=\"81\"/><rFont val=\"Tahoma\"/><family val=\"2\"/></rPr><t xml:space=\"preserve\">{}:\n</t></r>",
                escape_xml(&cm.autor)
            );
        }
        let _ = write!(
            lista,
            "<r><rPr><sz val=\"9\"/><color indexed=\"81\"/><rFont val=\"Tahoma\"/><family val=\"2\"/></rPr><t xml:space=\"preserve\">{}</t></r></text></comment>",
            escape_xml(&cm.texto)
        );
    }
    let autores: String = autores.iter().map(|a| format!("<author>{}</author>", escape_xml(a))).collect();
    let comentarios = format!("{XML_DECL}<comments xmlns=\"{NS}\"><authors>{autores}</authors><commentList>{lista}</commentList></comments>");

    let mut vml = format!(
        "<xml xmlns:v=\"urn:schemas-microsoft-com:vml\" xmlns:o=\"urn:schemas-microsoft-com:office:office\" xmlns:x=\"urn:schemas-microsoft-com:office:excel\">\
<o:shapelayout v:ext=\"edit\"><o:idmap v:ext=\"edit\" data=\"{n}\"/></o:shapelayout>\
<v:shapetype id=\"_x0000_t202\" coordsize=\"21600,21600\" o:spt=\"202\" path=\"m,l,21600r21600,l21600,xe\"><v:stroke joinstyle=\"miter\"/><v:path gradientshapeok=\"t\" o:connecttype=\"rect\"/></v:shapetype>"
    );
    for (i, (&(r, c), cm)) in sheet.comentarios.iter().enumerate() {
        let visibilidade = if cm.visivel { "visible" } else { "hidden" };
        let _ = write!(
            vml,
            "<v:shape id=\"_x0000_s{}\" type=\"#_x0000_t202\" style=\"position:absolute;margin-left:59.25pt;margin-top:1.5pt;width:108pt;height:59.25pt;z-index:{};visibility:{visibilidade}\" fillcolor=\"#ffffe1\" o:insetmode=\"auto\">\
<v:fill color2=\"#ffffe1\"/><v:shadow on=\"t\" color=\"black\" obscured=\"t\"/><v:path o:connecttype=\"none\"/><v:textbox style=\"mso-direction-alt:auto\"><div style=\"text-align:left\"></div></v:textbox>\
<x:ClientData ObjectType=\"Note\"><x:MoveWithCells/><x:SizeWithCells/><x:Anchor>{}, 15, {}, 2, {}, 15, {}, 4</x:Anchor><x:AutoFill>False</x:AutoFill><x:Row>{r}</x:Row><x:Column>{c}</x:Column>{}</x:ClientData></v:shape>",
            n * 1024 + i + 1,
            i + 1,
            c + 1,
            r.saturating_sub(1),
            c + 3,
            r + 3,
            if cm.visivel { "<x:Visible/>" } else { "" },
        );
    }
    vml.push_str("</xml>");
    Some((comentarios, vml))
}

#[cfg(test)]
mod testes {
    use feathersheet_core::planilha::{sheet::comentarios::Comentario, workbook::Workbook};

    /// As peças que o Excel exige para abrir sem "reparar": a parte de
    /// comentários e o VML declarados no [Content_Types], as duas relações
    /// da aba e o `<legacyDrawing>` no fim da aba.
    #[test]
    fn pacote_tem_tudo_que_o_excel_pede() {
        let mut wb = Workbook::new();
        wb.editar(0, |s| {
            s.definir_comentario(1, 1, Some(Comentario { autor: "Ana".into(), texto: "a < b".into(), visivel: true }));
        });
        let partes = crate::xlsx_write::montar_partes(&wb);
        let parte = |nome: &str| partes.iter().find(|(n, _)| n == nome)
            .map(|(_, b)| String::from_utf8(b.clone()).unwrap()).unwrap_or_else(|| panic!("falta {nome}"));
        let tipos = parte("[Content_Types].xml");
        assert!(tipos.contains("<Default Extension=\"vml\""));
        assert!(tipos.contains("PartName=\"/xl/comments1.xml\""));
        let aba = parte("xl/worksheets/sheet1.xml");
        assert!(aba.ends_with("<legacyDrawing r:id=\"rIdVmlNotas\"/></worksheet>"));
        let rels = parte("xl/worksheets/_rels/sheet1.xml.rels");
        assert!(rels.contains("Target=\"../comments1.xml\"") && rels.contains("Target=\"../drawings/vmlDrawing1.vml\""));
        let lista = parte("xl/comments1.xml");
        assert!(lista.contains("ref=\"B2\"") && lista.contains("a &lt; b"));
        let vml = parte("xl/drawings/vmlDrawing1.vml");
        assert!(vml.contains("<x:Row>1</x:Row><x:Column>1</x:Column><x:Visible/>"));
        // Aba sem comentário não ganha nada disso.
        let vazia = crate::xlsx_write::montar_partes(&Workbook::new());
        assert!(!vazia.iter().any(|(n, _)| n.contains("comments") || n.ends_with(".vml")));
    }
}
