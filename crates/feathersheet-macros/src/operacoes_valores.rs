//! Migração das operações de valor de src/macros/api-valores.ts.
//! A transação é aberta/fechada pelo host da macro. Este módulo apenas
//! trabalha nas células solicitadas, sem abrir novas transações.
use feathersheet_core::{
    celulas::refs::Rect,
    planilha::sheet::Sheet,
};

/// Apaga conteúdos (incluindo fórmulas), mas conserva estilos DIRETOS.
/// Os estilos de aba, linha e coluna são herdados e não são modificados.
/// Não aloca matriz do intervalo inteiro nem converte estilo em texto.
pub(crate) fn limpar_conteudo(sheet: &mut Sheet, area: &Rect) -> usize {
    let mut total = 0;
    for r in area.r0..=area.r1 {
        for c in area.c0..=area.c1 {
            let Some(mut anterior) = sheet.snapshot(r,c) else { continue };
            if anterior.raw.is_empty() { continue; }
            anterior.raw.clear();
            let proximo = if anterior.style.is_some() { Some(anterior) } else { None };
            if sheet.apply(r,c,proximo,true) { total += 1; }
        }
    }
    total
}

/// Apaga apenas a formatação DIRETA de cada célula.
/// Mantém valores, fórmulas, estilos herdados de linhas/colunas/aba.
pub(crate) fn limpar_formatos(sheet: &mut Sheet, area: &Rect) -> usize {
    let mut total = 0;
    for r in area.r0..=area.r1 {
        for c in area.c0..=area.c1 {
            let Some(mut anterior) = sheet.snapshot(r,c) else { continue };
            if anterior.style.is_none() { continue; }
            anterior.style = None;
            let proximo = if anterior.raw.is_empty() { None } else { Some(anterior) };
            if sheet.apply(r,c,proximo,true) { total += 1; }
        }
    }
    total
}

/// Troca todas as ocorrências literais, nunca interpreta a busca como regex.
/// Para case-insensitive compara Unicode por caractere; casos especiais de
/// case-folding expansivo (ß/İ) ainda precisam de paridade separada com JS.
fn substituir_texto(original: &str, antigo: &str, novo: &str, case_sensitive: bool)
    -> Option<String>
{
    if antigo.is_empty() { return None; }
    if case_sensitive {
        if !original.contains(antigo) { return None; }
        return Some(original.replace(antigo,novo));
    }
    let qtd = antigo.chars().count();
    if qtd == 0 { return None; }
    let procura = antigo.to_lowercase();
    let mut saida = String::with_capacity(original.len());
    let mut indice = 0;
    let mut mudou = false;
    while indice < original.len() {
        let trecho = &original[indice..];
        let fim = trecho.char_indices().nth(qtd)
            .map(|(pos,_)| pos).unwrap_or(trecho.len());
        let candidato = &trecho[..fim];
        if candidato.chars().count() == qtd && candidato.to_lowercase() == procura {
            saida.push_str(novo);
            indice += fim;
            mudou = true;
        } else {
            let caractere = trecho.chars().next().expect("indice UTF-8 valido");
            saida.push(caractere);
            indice += caractere.len_utf8();
        }
    }
    mudou.then_some(saida)
}

/// Substitui texto na representação digitada, inclusive dentro de fórmulas.
/// Devolve número de células afetadas (não número de ocorrências).
pub(crate) fn substituir(
    sheet: &mut Sheet, area: &Rect, antigo: &str, novo: &str,
    match_case: bool,
) -> usize {
    if antigo.is_empty() { return 0; }
    let mut alteradas = 0;
    for r in area.r0..=area.r1 {
        for c in area.c0..=area.c1 {
            let Some(mut anterior) = sheet.snapshot(r,c) else { continue };
            if anterior.raw.is_empty() { continue; }
            if let Some(resultado) = substituir_texto(&anterior.raw,antigo,novo,match_case) {
                if anterior.raw == resultado { continue; }
                anterior.raw = resultado;
                if sheet.apply(r,c,Some(anterior),true) {
                    alteradas += 1;
                }
            }
        }
    }
    alteradas
}

/// Texto exibido: valor calculado, número formatado e erros da fórmula.
/// Compartilhado entre getText e find, sem cópia do algoritmo de formatação.
pub(crate) fn texto_exibido(
    pasta: &feathersheet_core::planilha::workbook::Workbook,
    aba: usize, linha: u32, coluna: u32,
) -> String {
    use feathersheet_core::celulas::{style::format_value, values::Value};
    match pasta.valor(aba,linha,coluna) {
        Value::Empty => String::new(),
        Value::Number(n) => {
            let estilo=pasta.aba(aba).and_then(|a|a.sheet.estilo_resolvido(linha,coluna));
            // format_value respeita o código personalizado (Style.code).
            format_value(&Value::Number(n),estilo.as_ref())
        }
        Value::Text(s) => s,
        Value::Bool(b) => if b { "VERDADEIRO" } else { "FALSO" }.into(),
        Value::Error(e) => e.code.as_str().into(),
    }
}

/// Como a propriedade MacroRange.address original: sempre coordenadas A1
/// completas, inclusive para linhas e colunas inteiras.
pub(crate) fn endereco_a1(area: &Rect) -> String {
    use feathersheet_core::celulas::refs::cell_name;
    let inicio=cell_name(area.r0,area.c0);
    if area.r0==area.r1 && area.c0==area.c1 { inicio }
    else { format!("{inicio}:{}",cell_name(area.r1,area.c1)) }
}

/// Primeira ocorrência em ordem linha/coluna, igual ao "find" do
/// src/macros/api-valores.ts. Não materializa um vetor de resultados.
pub(crate) fn localizar(
    pasta: &feathersheet_core::planilha::workbook::Workbook,
    aba: usize,
    area: &Rect,
    texto: &str,
    match_case: bool,
    whole_cell: bool,
    in_formulas: bool,
) -> Option<String> {
    use feathersheet_core::celulas::refs::cell_name;
    let sheet = &pasta.aba(aba)?.sheet;
    let consulta = if match_case { texto.into() } else { texto.to_lowercase() };
    for r in area.r0..=area.r1 {
        for c in area.c0..=area.c1 {
            if !sheet.has_content(r,c) { continue; }
            let exibido = if in_formulas {
                sheet.get_raw(r,c)
            } else {
                texto_exibido(pasta,aba,r,c)
            };
            let comparavel = if match_case { exibido } else { exibido.to_lowercase() };
            let achou = if whole_cell {
                comparavel == consulta
            } else {
                comparavel.contains(&consulta)
            };
            if achou { return Some(cell_name(r,c)); }
        }
    }
    None
}

#[cfg(test)]
mod testes {
    use super::*;
    use feathersheet_core::celulas::estilo_parcial::{Campo, EstiloParcial};
    #[test]
    fn clear_contents_mantem_formato_e_desfaz_como_uma_operacao() {
        let mut s=Sheet::default();
        s.set_cell(0,0,"=2+3");
        let mut st=EstiloParcial::new();
        st.insert("bold".into(),Campo::Logico(true));
        s.set_style(&Rect::cell(0,0),&st);
        s.abrir_transacao();
        assert_eq!(limpar_conteudo(&mut s,&Rect::cell(0,0)),1);
        s.fechar_transacao();
        assert_eq!(s.get_raw(0,0),"");
        assert!(s.estilo_mesclado(0,0).is_some());
        assert!(s.undo());
        assert_eq!(s.get_raw(0,0),"=2+3");
    }
    #[test]
    fn clear_formats_conserva_valor_e_formula() {
        let mut s=Sheet::default();
        s.set_cell(0,0,"42");
        let mut st=EstiloParcial::new();
        st.insert("bold".into(),Campo::Logico(true));
        s.set_style(&Rect::cell(0,0),&st);
        s.abrir_transacao();
        assert_eq!(limpar_formatos(&mut s,&Rect::cell(0,0)),1);
        s.fechar_transacao();
        assert_eq!(s.get_raw(0,0),"42");
        assert!(s.estilo_mesclado(0,0).is_none());
    }
    #[test]
    fn substitui_texto_literal_sem_interpretar_regex() {
        let mut s=Sheet::default();
        s.set_cell(0,0,"Abril.ABRIL.abril");
        s.set_cell(1,0,"=SE(A1=\"abril\";1;0)");
        s.abrir_transacao();
        assert_eq!(substituir(&mut s,&Rect::new(0,0,1,0),"abril","maio",false),2);
        s.fechar_transacao();
        assert_eq!(s.get_raw(0,0),"maio.maio.maio");
        assert_eq!(s.get_raw(1,0),"=SE(A1=\"maio\";1;0)");
        assert!(s.undo());
        assert_eq!(s.get_raw(0,0),"Abril.ABRIL.abril");
    }
    #[test]
    fn find_compara_resultado_ou_formula_e_mantem_ordem_de_leitura() {
        use feathersheet_core::planilha::workbook::Workbook;
        let mut pasta=Workbook::new();
        pasta.escrever_celula(0,1,1,"Projeto Abril".into());
        pasta.escrever_celula(0,0,2,"=1+2".into());
        let area=Rect::new(0,0,3,3);
        assert_eq!(localizar(&pasta,0,&area,"3",false,false,false),Some("C1".into()));
        assert_eq!(localizar(&pasta,0,&area,"=1+2",true,true,true),Some("C1".into()));
        assert_eq!(localizar(&pasta,0,&area,"abril",false,false,false),Some("B2".into()));
        assert_eq!(localizar(&pasta,0,&area,"Abril",true,true,false),None);
        assert_eq!(localizar(&pasta,0,&area,"nada",false,false,false),None);
    }

    #[test]
    fn nao_cria_celulas_vazias_sem_valor_ou_estilo() {
        let mut s=Sheet::default();
        s.abrir_transacao();
        assert_eq!(limpar_conteudo(&mut s,&Rect::new(0,0,12,12)),0);
        assert_eq!(limpar_formatos(&mut s,&Rect::new(0,0,12,12)),0);
        assert_eq!(substituir(&mut s,&Rect::new(0,0,12,12),"a","b",false),0);
        s.fechar_transacao();
        assert_eq!(s.len(),0);
    }
}
