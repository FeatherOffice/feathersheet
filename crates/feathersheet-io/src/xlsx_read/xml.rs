//! Árvore de XML pequena, para as partes do `.xlsx` — espelha `parseXml`/
//! `child`/`children` de `src/io/xlsx.ts:33-160`. Usa a crate `quick-xml`
//! (não um analisador à mão como o TypeScript: lá não havia dependência
//! disponível; aqui há, e o alvo são arquivos reais bem formados).
//!
//! Nome dos nós vem sem prefixo de namespace (`local_name`), igual ao JS —
//! o projeto não resolve namespace de verdade, só casa pelo nome local.
//! Atributo mantém o nome como veio (com prefixo, ex. `r:id`), também igual
//! ao JS, porque o resto do código lê `attrs.get("r:id")` direto.

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;
use std::io::BufRead;

#[derive(Debug, Default, Clone)]
pub struct XmlNode {
    pub name: String,
    pub attrs: HashMap<String, String>,
    pub children: Vec<XmlNode>,
    pub text: String,
}

impl XmlNode {
    pub fn child(&self, name: &str) -> Option<&XmlNode> {
        self.children.iter().find(|c| c.name == name)
    }

    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a XmlNode> {
        self.children.iter().filter(move |c| c.name == name)
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.get(name).map(|s| s.as_str())
    }
}

fn local_name(raw: &[u8]) -> String {
    let s = String::from_utf8_lossy(raw);
    match s.find(':') {
        Some(i) => s[i + 1..].to_string(),
        None => s.to_string(),
    }
}

fn read_attrs(e: &quick_xml::events::BytesStart, decoder: quick_xml::Decoder) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    for a in e.attributes().flatten() {
        let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
        let val = a
            .decode_and_unescape_value(decoder)
            .map(|c| c.to_string())
            .unwrap_or_default();
        attrs.insert(key, val);
    }
    attrs
}

/// Faz o mesmo que `parseXml` do TypeScript: devolve o primeiro elemento de
/// topo (um documento com vários elementos de topo, XML inválido mas
/// tolerado, só olha o primeiro). Erro só em XML realmente quebrado (tag
/// nunca fechada em condições que o `quick-xml` não consegue recuperar).
pub fn parse_xml(src: &str) -> Result<XmlNode, String> {
    parse_xml_impl(Reader::from_reader(src.as_bytes()), None)
}

/// Lê a mesma estrutura do XLSX, mas emite cada `row` de `sheetData`
/// imediatamente para o consumidor, em vez de acumular dezenas de milhares
/// de nós XML até o fim da planilha. Os outros nós (cols, links, mesclas,
/// condicional) continuam na árvore e são tratados pelo mesmo importador.
/// A API geral `parse_xml` continua com seu comportamento original.
pub fn parse_xml_linhas(
    src: &str,
    callback: &mut dyn FnMut(&XmlNode),
) -> Result<XmlNode, String> {
    parse_xml_items_reader(src.as_bytes(), "sheetData", "row", callback)
}

/// Lê diretamente de um fluxo ZIP sem montar a entrada XML inteira na RAM.
pub fn parse_xml_items_reader<R: BufRead>(
    source: R,
    parent: &str,
    item: &str,
    callback: &mut dyn FnMut(&XmlNode),
) -> Result<XmlNode, String> {
    let mut adapter = |node: &XmlNode| { callback(node); Ok(()) };
    parse_xml_items_reader_result(source, parent, item, &mut adapter)
}

/// Variante cancelável: interrompe a descompressão assim que o consumidor
/// detectar excesso de células/strings, sem varrer gigabytes até o EOF.
pub fn parse_xml_items_reader_result<R: BufRead>(
    source: R,
    parent: &str,
    item: &str,
    callback: &mut dyn FnMut(&XmlNode) -> Result<(), String>,
) -> Result<XmlNode, String> {
    parse_xml_impl(Reader::from_reader(source), Some((parent, item, callback)))
}

fn parse_xml_impl<R: BufRead>(
    mut reader: Reader<R>,
    mut callback: Option<(&str, &str, &mut dyn FnMut(&XmlNode) -> Result<(), String>)>,
) -> Result<XmlNode, String> {
    reader.config_mut().trim_text(false);
    let mut stack: Vec<XmlNode> = vec![XmlNode::default()];
    let mut buf = Vec::new();

    loop {
        let decoder = reader.decoder();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let node = XmlNode {
                    name: local_name(e.name().as_ref()),
                    attrs: read_attrs(&e, decoder),
                    ..Default::default()
                };
                stack.push(node);
            }
            Ok(Event::Empty(e)) => {
                let node = XmlNode {
                    name: local_name(e.name().as_ref()),
                    attrs: read_attrs(&e, decoder),
                    ..Default::default()
                };
                let emitir = callback.as_ref().is_some_and(|(pai, item, _)|
                    stack.last().is_some_and(|n| n.name == *pai) && node.name == *item
                );
                if emitir {
                    if let Some((_, _, ler)) = callback.as_mut() { ler(&node)?; }
                } else if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                }
            }
            Ok(Event::End(_)) => {
                if stack.len() > 1 {
                    let node = stack.pop().expect("verificado acima");
                    let emitir = callback.as_ref().is_some_and(|(pai, item, _)|
                        stack.last().is_some_and(|n| n.name == *pai) && node.name == *item
                    );
                    if emitir {
                        if let Some((_, _, ler)) = callback.as_mut() { ler(&node)?; }
                    } else {
                        stack.last_mut().expect("raiz sempre existe").children.push(node);
                    }
                }
            }
            Ok(Event::Text(e)) => {
                let txt = e.decode().unwrap_or_default();
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&txt);
                }
            }
            // `&amp;`, `&#233;` etc. chegam como evento à parte (não dentro
            // do Text), diferente do parser à mão do JS — resolve aqui pra
            // ficar igual: 5 entidades nomeadas + referência numérica.
            Ok(Event::GeneralRef(e)) => {
                let resolved = e.resolve_char_ref().ok().flatten().map(|c| c.to_string()).or_else(|| {
                    let name = e.decode().ok()?;
                    match name.as_ref() {
                        "lt" => Some("<".to_string()),
                        "gt" => Some(">".to_string()),
                        "amp" => Some("&".to_string()),
                        "quot" => Some("\"".to_string()),
                        "apos" => Some("'".to_string()),
                        _ => None,
                    }
                });
                if let (Some(top), Some(txt)) = (stack.last_mut(), resolved) {
                    top.text.push_str(&txt);
                }
            }
            Ok(Event::CData(e)) => {
                let txt = String::from_utf8_lossy(e.as_ref()).to_string();
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&txt);
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {} // comentário, PI, declaração — ignorados, igual ao JS
            Err(e) => return Err(format!("XML inválido: {e}")),
        }
        buf.clear();
    }

    let mut root = stack.pop().unwrap_or_default();
    Ok(if root.children.is_empty() {
        root
    } else {
        root.children.remove(0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_cancela_fluxo_ao_receber_erro_no_callback() {
        // A parte final é XML inválido: o parser deve parar no primeiro row
        // e devolver o erro solicitado, sem tentar ler o restante do ZIP.
        let xml = "<worksheet><sheetData><row r=\"1\"/></sheetData><invalida";
        let erro = parse_xml_items_reader_result(
            xml.as_bytes(), "sheetData", "row", &mut |_| Err("limite atingido".into()),
        ).unwrap_err();
        assert_eq!(erro, "limite atingido");
    }

    #[test]
    fn leitor_stream_de_itens_preserva_metadados_e_descarta_so_as_linhas() {
        use std::io::{BufReader, Cursor};
        let xml = br#"<worksheet><sheetData><row r="1"><c r="A1"><v>7</v></c></row><row r="2"/></sheetData><mergeCells><mergeCell ref="A1:A2"/></mergeCells></worksheet>"#;
        let mut rows = Vec::new();
        // BufRead pequeno: confirma que o leitor nao depende de todo o XML
        // estar numa unica String em memoria, como ocorre num arquivo ZIP.
        let source = BufReader::with_capacity(7, Cursor::new(xml));
        let root = parse_xml_items_reader(source, "sheetData", "row", &mut |row| {
            rows.push(row.attr("r").unwrap_or("").to_owned());
        }).unwrap();
        assert_eq!(rows, ["1", "2"]);
        assert!(root.child("sheetData").unwrap().children.is_empty());
        assert_eq!(root.child("mergeCells").unwrap().children.len(), 1);
    }

    #[test]
    fn leitor_incremental_de_linhas_descarta_a_arvore_de_celulas() {
        let mut linhas = Vec::new();
        let xml = r#"<worksheet><cols><col min="1" max="1" width="30"/></cols><sheetData><row r="1"><c r="A1"><v>42</v></c></row><row r="2"><c r="B2"><v>91</v></c></row><row r="3"/></sheetData><mergeCells><mergeCell ref="A1:B1"/></mergeCells></worksheet>"#;
        let arvore = parse_xml_linhas(xml, &mut |linha| {
            linhas.push((linha.attr("r").unwrap_or("").to_string(), linha.children.len()));
        }).unwrap();
        assert_eq!(linhas, vec![("1".into(), 1), ("2".into(), 1), ("3".into(), 0)]);
        assert!(arvore.child("sheetData").unwrap().children.is_empty());
        assert_eq!(arvore.child("mergeCells").unwrap().children.len(), 1);
        assert_eq!(arvore.child("cols").unwrap().children.len(), 1);
        // O parser genérico não deve descartar linhas!
        assert_eq!(parse_xml(xml).unwrap().child("sheetData").unwrap().children.len(), 3);
    }

    #[test]
    fn atributos_texto_e_aninhamento() {
        let xml = r#"<sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1"/></row></sheetData>"#;
        let root = parse_xml(xml).unwrap();
        assert_eq!(root.name, "sheetData");
        let row = root.child("row").unwrap();
        assert_eq!(row.attr("r"), Some("1"));
        let cells: Vec<_> = row.children_named("c").collect();
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0].attr("t"), Some("s"));
        assert_eq!(cells[0].child("v").unwrap().text, "0");
        assert_eq!(cells[1].attr("r"), Some("B1"));
    }

    #[test]
    fn entidades_e_cdata_voltam_ao_texto_original() {
        let xml = r#"<t>a &amp; b &lt;c&gt; &#233; &#x41;<![CDATA[<cru>]]></t>"#;
        let root = parse_xml(xml).unwrap();
        assert_eq!(root.text, "a & b <c> é A<cru>");
    }

    #[test]
    fn comentario_e_prologo_nao_viram_no() {
        let xml = r#"<?xml version="1.0"?><raiz><!-- comentário --><a/></raiz>"#;
        let root = parse_xml(xml).unwrap();
        assert_eq!(root.name, "raiz");
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].name, "a");
    }

    #[test]
    fn namespace_casa_pelo_nome_local() {
        let xml = r#"<r:root xmlns:r="urn:x"><r:id>5</r:id></r:root>"#;
        let root = parse_xml(xml).unwrap();
        assert_eq!(root.name, "root");
        assert_eq!(root.child("id").unwrap().text, "5");
    }
}
