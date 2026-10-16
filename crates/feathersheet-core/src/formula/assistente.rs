//! O que ajuda a escrever uma fórmula: as referências coloridas do texto, a
//! função que envolve o cursor, as sugestões de nome, onde um clique pode
//! virar referência, e reescrever a referência arrastada. Mais o catálogo das
//! funções (categorias, apelidos, as que não têm ajuda).
//!
//! Tradução do `assist.ts`, da parte de texto do `ref-drag.ts` e do resto do
//! `function-help.ts`. Tudo é conta sobre o texto da fórmula, sem planilha.
//!
//! ⚠ Posições são contadas em **caracteres**, e o JavaScript conta em
//! unidades UTF-16. Para o que aparece numa fórmula (acento, cifrão, letra
//! grega) os dois batem; emoji é o único caso em que não bateriam — a mesma
//! convenção do `ParseError.at`.

use std::cmp::Ordering;

use crate::celulas::refs::{col_name, parse_range_ref, Rect};
use crate::funcoes::function_help::{help_for, signature, FunctionHelp, CATEGORIES};
use crate::funcoes::functions::{aliases, canonical, function_names};

/// As cores das referências, na ordem em que aparecem (`REF_COLORS`).
pub const CORES: [&str; 6] = ["#2b68d6", "#129d6b", "#d97706", "#9333ea", "#dc2626", "#0891b2"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug)]
pub struct Sugestao {
    pub name: String,
    pub help: Option<&'static FunctionHelp>,
    pub signature: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chamada {
    pub name: String,
    /// Índice do argumento em que o cursor está (0 = primeiro).
    pub arg_index: usize,
    /// Posição do "(" que abriu a chamada.
    pub open: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Referencia {
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub color: &'static str,
}

/// Os cifrões de cada ponta, para reescrever sem perder as travas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Travas {
    pub c0: bool,
    pub r0: bool,
    pub c1: bool,
    pub r1: bool,
}

/// Uma referência da fórmula com a área que ela aponta (`RefBox`).
#[derive(Clone, Debug, PartialEq)]
pub struct Caixa {
    pub start: usize,
    pub end: usize,
    /// A aba escrita antes (`Dados!A1`), quando há.
    pub sheet: Option<String>,
    pub rect: Rect,
    pub color: &'static str,
    pub locks: Travas,
}

/// `/[A-Za-zÀ-ÿ0-9_.]/`
fn ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '.' || ('\u{c0}'..='\u{ff}').contains(&c)
}

fn letras(t: &str) -> Vec<char> {
    t.chars().collect()
}

fn pedaco(l: &[char], a: usize, b: usize) -> String {
    l[a.min(l.len())..b.min(l.len())].iter().collect()
}

/// O identificador digitado logo antes do cursor.
///
/// Esquisitice do TypeScript, mantida: com o cursor além do fim do texto,
/// `text[i]` é `undefined` e o `IDENT.test(undefined)` testa a palavra
/// "undefined" — que casa. Então cada posição além do fim conta como letra, e
/// o token termina no cursor, não no fim do texto (`=SOMA(` com cursor 10 dá
/// um token vazio de 6 a 10). A interface nunca manda cursor além do fim.
pub fn token_before(texto: &str, cursor: usize) -> Option<Token> {
    let l = letras(texto);
    let mut ini = cursor;
    while ini > 0 && (ini > l.len() || ident(l[ini - 1])) {
        ini -= 1;
    }
    (ini != cursor).then(|| Token { text: pedaco(&l, ini, cursor), start: ini, end: cursor })
}

/// `/^\$?[A-Za-z]{1,3}\$?\d+$/`
fn parece_referencia(t: &str) -> bool {
    let b = t.as_bytes();
    let mut i = usize::from(b.first() == Some(&b'$'));
    let ini = i;
    while i < b.len() && b[i].is_ascii_alphabetic() {
        i += 1;
    }
    if !(1..=3).contains(&(i - ini)) {
        return false;
    }
    if b.get(i) == Some(&b'$') {
        i += 1;
    }
    i < b.len() && b[i..].iter().all(u8::is_ascii_digit)
}

/// As sugestões de função para o que está sendo digitado (`suggestAt`).
pub fn suggest_at(texto: &str, cursor: usize, limite: usize) -> Vec<Sugestao> {
    if !texto.starts_with('=') {
        return Vec::new();
    }
    let Some(token) = token_before(texto, cursor) else { return Vec::new() };
    if token.start == 0 || parece_referencia(&token.text) {
        return Vec::new();
    }
    // Já está dentro dos parênteses desta função: não sugerir de novo.
    if letras(texto).get(token.end) == Some(&'(') {
        return Vec::new();
    }
    suggest(&token.text, limite)
}

pub fn suggest(prefixo: &str, limite: usize) -> Vec<Sugestao> {
    let maiusculo = prefixo.to_uppercase();
    if maiusculo.is_empty() {
        return Vec::new();
    }
    let mut nomes: Vec<&str> = function_names().into_iter().filter(|n| n.starts_with(&maiusculo)).collect();
    nomes.sort_by(|a, b| a.chars().count().cmp(&b.chars().count()).then_with(|| comparar_pt(a, b)));
    nomes.into_iter().take(limite).map(describe).collect()
}

pub fn describe(nome: &str) -> Sugestao {
    let help = ajuda(nome);
    Sugestao {
        name: nome.to_string(),
        help,
        signature: help.map(|h| signature(nome, h)).unwrap_or_else(|| format!("{nome}()")),
    }
}

/// A ajuda de uma função pelo nome em inglês ou em português (`helpFor`).
pub fn ajuda(nome: &str) -> Option<&'static FunctionHelp> {
    let maiusculo = nome.to_uppercase();
    help_for(&maiusculo).or_else(|| canonical(&maiusculo).and_then(help_for))
}

/// A chamada de função que envolve o cursor (`activeCall`): considera
/// aninhamento e ignora texto entre aspas.
pub fn active_call(texto: &str, cursor: usize) -> Option<Chamada> {
    let l = letras(texto);
    let mut pilha: Vec<Chamada> = Vec::new();
    let mut em_texto = false;
    let mut i = 0;
    while i < cursor && i < l.len() {
        let c = l[i];
        if em_texto {
            if c == '"' {
                if l.get(i + 1) == Some(&'"') {
                    i += 1; // aspas escapadas continuam dentro do texto
                } else {
                    em_texto = false;
                }
            }
        } else if c == '"' {
            em_texto = true;
        } else if c == '(' {
            let mut ini = i;
            while ini > 0 && ident(l[ini - 1]) {
                ini -= 1;
            }
            pilha.push(Chamada { name: pedaco(&l, ini, i), arg_index: 0, open: i });
        } else if c == ')' {
            pilha.pop();
        } else if c == ',' || c == ';' {
            if let Some(topo) = pilha.last_mut() {
                topo.arg_index += 1;
            }
        }
        i += 1;
    }
    pilha.into_iter().rev().find(|c| !c.name.is_empty())
}

/// Se o clique numa célula pode virar referência nesta posição do texto
/// (`acceptsReference`): logo depois de um operador, de "(", de ";" ou do "=".
pub fn accepts_reference(texto: &str, cursor: usize) -> bool {
    if !texto.starts_with('=') {
        return false;
    }
    let l = letras(texto);
    let antes: String = pedaco(&l, 0, cursor);
    let antes = antes.trim_end();
    if antes.is_empty() || antes == "=" {
        return true;
    }
    if aspas_sem_escape(antes) % 2 == 1 {
        return false;
    }
    antes.ends_with(['=', '+', '-', '*', '/', '^', '&', '<', '>', '(', ',', ';', ':'])
}

fn aspas_sem_escape(t: &str) -> usize {
    let l = letras(t);
    let (mut n, mut i) = (0, 0);
    while i < l.len() {
        if l[i] == '"' {
            if l.get(i + 1) == Some(&'"') {
                i += 1;
            } else {
                n += 1;
            }
        }
        i += 1;
    }
    n
}

fn dentro_de_texto(l: &[char], indice: usize) -> bool {
    let (mut aberto, mut i) = (false, 0);
    while i < indice {
        if l[i] == '"' {
            if l.get(i + 1) == Some(&'"') {
                i += 1;
            } else {
                aberto = !aberto;
            }
        }
        i += 1;
    }
    aberto
}

/// Casa `\$?[A-Za-z]{1,3}\$?\d{1,8}` a partir de `i`; devolve onde termina.
fn casar_canto(l: &[char], mut i: usize) -> Option<usize> {
    if l.get(i) == Some(&'$') {
        i += 1;
    }
    let ini = i;
    while i < l.len() && l[i].is_ascii_alphabetic() {
        i += 1;
    }
    // Mais de três letras seguidas não casam em ponto nenhum desta posição:
    // depois de três, o que vem é letra, e não cifrão nem algarismo.
    if !(1..=3).contains(&(i - ini)) {
        return None;
    }
    if l.get(i) == Some(&'$') {
        i += 1;
    }
    let dig = i;
    while i < l.len() && i - dig < 8 && l[i].is_ascii_digit() {
        i += 1;
    }
    (i > dig).then_some(i)
}

/// Casa a expressão inteira das referências a partir de `i`, com a segunda
/// ponta opcional (`(\s*:\s*canto)?`).
fn casar_referencia(l: &[char], i: usize) -> Option<usize> {
    let fim = casar_canto(l, i)?;
    let mut j = fim;
    while j < l.len() && l[j].is_whitespace() {
        j += 1;
    }
    if l.get(j) != Some(&':') {
        return Some(fim);
    }
    j += 1;
    while j < l.len() && l[j].is_whitespace() {
        j += 1;
    }
    Some(casar_canto(l, j).unwrap_or(fim))
}

/// As referências A1 e os intervalos da fórmula (`referencesIn`).
pub fn references_in(texto: &str) -> Vec<(String, usize, usize)> {
    let l = letras(texto);
    let mut saida = Vec::new();
    let mut i = 0;
    while i < l.len() {
        let Some(fim) = casar_referencia(&l, i) else {
            i += 1;
            continue;
        };
        // Como o `matchAll`: a próxima procura começa depois deste trecho,
        // mesmo quando ele é descartado abaixo.
        let antes = i.checked_sub(1).map(|k| l[k]);
        let depois = l.get(fim).copied();
        let descarta = antes.is_some_and(ident) || depois.is_some_and(|d| d == '(' || ident(d)) || dentro_de_texto(&l, i);
        if !descarta {
            saida.push((pedaco(&l, i, fim), i, fim));
        }
        i = fim;
    }
    saida
}

/// Cada referência com uma cor: a mesma célula tem sempre a mesma cor.
pub fn color_references(texto: &str) -> Vec<Referencia> {
    let mut vistas: Vec<(String, &'static str)> = Vec::new();
    references_in(texto)
        .into_iter()
        .map(|(text, start, end)| {
            let canonica = text.replace('$', "").to_uppercase();
            let color = match vistas.iter().find(|(c, _)| *c == canonica) {
                Some((_, cor)) => cor,
                None => {
                    let cor = CORES[vistas.len() % CORES.len()];
                    vistas.push((canonica, cor));
                    cor
                }
            };
            Referencia { text, start, end, color }
        })
        .collect()
}

/// `/^(\$?)([A-Za-z]{1,3})(\$?)(\d{1,7})$/`: as travas de uma ponta.
fn travas_da_ponta(t: &str) -> Option<(bool, bool)> {
    let l = letras(t);
    let fim = casar_canto(&l, 0)?;
    if fim != l.len() {
        return None;
    }
    let trava_col = l.first() == Some(&'$');
    let trava_lin = l.iter().skip(1).any(|c| *c == '$');
    Some((trava_col, trava_lin))
}

/// As referências da fórmula com a área de cada uma (`referenceBoxes`).
pub fn reference_boxes(texto: &str) -> Vec<Caixa> {
    let l = letras(texto);
    color_references(texto)
        .into_iter()
        .filter_map(|r| {
            let rect = parse_range_ref(&r.text)?;
            let mut pontas = r.text.split(':').map(str::trim);
            let a = travas_da_ponta(pontas.next()?)?;
            let b = match pontas.next() {
                Some(p) => travas_da_ponta(p)?,
                None => a,
            };
            Some(Caixa {
                start: r.start,
                end: r.end,
                sheet: aba_antes(&l, r.start),
                rect,
                color: r.color,
                locks: Travas { c0: a.0, r0: a.1, c1: b.0, r1: b.1 },
            })
        })
        .collect()
}

/// O nome da aba escrito logo antes da referência (`Dados!A1` e `'Meu Plano'!A1`).
fn aba_antes(l: &[char], inicio: usize) -> Option<String> {
    if inicio < 2 || l[inicio - 1] != '!' {
        return None;
    }
    let i = inicio - 2;
    if l[i] == '\'' {
        // Anda para trás até a aspa de abertura; '' é uma aspa no meio do nome.
        let mut j = i as isize - 1;
        while j >= 0 {
            if l[j as usize] == '\'' {
                if j >= 1 && l[j as usize - 1] == '\'' {
                    j -= 2;
                    continue;
                }
                break;
            }
            j -= 1;
        }
        if j < 0 {
            return None;
        }
        return Some(pedaco(l, j as usize + 1, i).replace("''", "'"));
    }
    let nome_ok = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '.' || ('\u{c0}'..='\u{24f}').contains(&c);
    let mut j = i as isize;
    while j >= 0 && nome_ok(l[j as usize]) {
        j -= 1;
    }
    let nome = pedaco(l, (j + 1) as usize, i + 1);
    (!nome.is_empty()).then_some(nome)
}

/// A referência escrita de volta, com as travas que ela já tinha (`formatBox`).
pub fn format_box(rect: &Rect, t: Travas) -> String {
    let canto = |tc: bool, tl: bool, c: u32, r: u32| {
        format!("{}{}{}{}", if tc { "$" } else { "" }, col_name(c), if tl { "$" } else { "" }, r + 1)
    };
    let inicio = canto(t.c0, t.r0, rect.c0, rect.r0);
    if rect.r0 == rect.r1 && rect.c0 == rect.c1 {
        return inicio;
    }
    format!("{inicio}:{}", canto(t.c1, t.r1, rect.c1, rect.r1))
}

/// A fórmula com o trecho `inicio..fim` trocado pela área nova (`rewriteBox`).
pub fn rewrite_box(texto: &str, inicio: usize, fim: usize, travas: Travas, rect: &Rect) -> String {
    let l = letras(texto);
    format!("{}{}{}", pedaco(&l, 0, inicio), format_box(rect, travas), pedaco(&l, fim, l.len()))
}

/// Alterna as travas da referência sob o cursor, como o F4 das planilhas.
/// A ordem é relativa, absoluta, coluna absoluta e linha absoluta.
pub fn toggle_reference(texto: &str, cursor: usize) -> Option<String> {
    let caixa = reference_boxes(texto).into_iter().find(|caixa| caixa.start <= cursor && cursor <= caixa.end)?;
    let proximas = if caixa.locks.c0 && caixa.locks.r0 {
        Travas { c0: false, r0: true, c1: false, r1: true }
    } else if !caixa.locks.c0 && caixa.locks.r0 {
        Travas { c0: true, r0: false, c1: true, r1: false }
    } else if caixa.locks.c0 && !caixa.locks.r0 {
        Travas::default()
    } else {
        Travas { c0: true, r0: true, c1: true, r1: true }
    };
    Some(rewrite_box(texto, caixa.start, caixa.end, proximas, &caixa.rect))
}

// ------------------------------------------------------------- o catálogo

/// A letra sem acento, só para comparar grafias (`normalize('NFD')` sem as
/// marcas).
fn sem_acento(c: char) -> char {
    const DE: &str = "ÀÁÂÃÄÅàáâãäåÇçÈÉÊËèéêëÌÍÎÏìíîïÑñÒÓÔÕÖòóôõöÙÚÛÜùúûüÝýÿ";
    const PARA: &str = "AAAAAAaaaaaaCcEEEEeeeeIIIIiiiiNnOOOOOoooooUUUUuuuuYyy";
    match DE.chars().position(|d| d == c) {
        Some(i) => PARA.chars().nth(i).unwrap_or(c),
        None => c,
    }
}

fn dobrar(t: &str) -> String {
    t.chars().map(sem_acento).collect()
}

/// A ordem do `localeCompare(…, 'pt-BR')` para os nomes de função: primeiro
/// pela letra sem acento e sem caixa (pontuação antes de algarismo, algarismo
/// antes de letra), depois o sem acento antes do acentuado, depois a
/// minúscula antes da maiúscula.
pub fn comparar_pt(a: &str, b: &str) -> Ordering {
    fn classe(c: char) -> (u8, char) {
        let base = sem_acento(c).to_ascii_lowercase();
        if base.is_ascii_alphabetic() {
            (2, base)
        } else if base.is_ascii_digit() {
            (1, base)
        } else {
            (0, base)
        }
    }
    let primaria = |t: &str| t.chars().map(classe).collect::<Vec<_>>();
    primaria(a)
        .cmp(&primaria(b))
        .then_with(|| {
            let acentos = |t: &str| t.chars().map(|c| sem_acento(c) != c).collect::<Vec<_>>();
            acentos(a).cmp(&acentos(b))
        })
        .then_with(|| {
            let caixa = |t: &str| t.chars().map(|c| c.is_uppercase()).collect::<Vec<_>>();
            caixa(a).cmp(&caixa(b))
        })
}

/// Os apelidos em português de um nome canônico (`aliasesOf`). Quando o
/// mesmo nome existe com e sem acento (MEDIA e MÉDIA), fica o acentuado.
pub fn aliases_of(canonico: &str) -> Vec<&'static str> {
    let implementadas = function_names();
    let mut preferidos: Vec<(String, &'static str)> = Vec::new();
    for &(apelido, real) in aliases() {
        // Como o `CANONICAL` do TypeScript: só o apelido de função que existe.
        if real != canonico || apelido == canonico || !implementadas.contains(&real) {
            continue;
        }
        let chave = dobrar(apelido);
        match preferidos.iter_mut().find(|(k, _)| *k == chave) {
            Some((_, atual)) => {
                if dobrar(atual) == *atual && dobrar(apelido) != apelido {
                    *atual = apelido;
                }
            }
            None => preferidos.push((chave, apelido)),
        }
    }
    let mut saida: Vec<&'static str> = preferidos.into_iter().map(|(_, a)| a).collect();
    saida.sort_by(|a, b| comparar_pt(a, b));
    saida
}

/// As funções registradas que não têm ajuda (`functionsWithoutHelp`).
pub fn functions_without_help() -> Vec<&'static str> {
    let implementadas = function_names();
    let documentada = |nome: &str| {
        help_for(nome).is_some()
            || aliases().iter().any(|&(a, r)| a == nome && implementadas.contains(&r) && help_for(r).is_some())
    };
    let mut saida: Vec<&'static str> = implementadas.iter().copied().filter(|n| !documentada(n)).collect();
    saida.sort_by(|a, b| comparar_pt(a, b));
    saida
}

/// As categorias do catálogo, como o `CATEGORIES` do TypeScript.
pub fn categorias() -> &'static [(&'static str, &'static [&'static str])] {
    CATEGORIES
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn referencias_coloridas_e_descartes() {
        let r: Vec<_> = color_references("=SOMA(A1:B2;$C$3)+A1+LOG10(2)+\"D4\"+ABCD1")
            .into_iter()
            .map(|r| (r.text, r.color))
            .collect();
        assert_eq!(r, vec![("A1:B2".into(), CORES[0]), ("$C$3".into(), CORES[1]), ("A1".into(), CORES[2])]);
    }

    #[test]
    fn chamada_ativa_e_referencia_aceita() {
        let c = active_call("=SE(A1>0;SOMA(B1;", 17).unwrap();
        assert_eq!((c.name.as_str(), c.arg_index), ("SOMA", 1));
        assert!(accepts_reference("=A1+", 4));
        assert!(!accepts_reference("=\"a", 3));
        assert!(!accepts_reference("=A1", 3));
    }

    // Os casos de "assistência de fórmulas" do `test/core/engine.test.ts`.

    #[test]
    fn sugere_funcoes_pelo_prefixo_digitado() {
        let nomes = |t: &str| suggest_at(t, t.chars().count(), 8).into_iter().map(|s| s.name).collect::<Vec<_>>();
        assert!(nomes("=PROC").contains(&"PROCV".to_string()));
        assert!(nomes("=PROC").contains(&"PROCX".to_string()));
        assert!(nomes("=SOM").contains(&"SOMA".to_string()));
        assert!(nomes("=VLOO").contains(&"VLOOKUP".to_string()));
        assert!(nomes("123").is_empty(), "não é fórmula");
        assert!(nomes("=A1").is_empty(), "parece referência");
        assert!(nomes("=SOMA(").is_empty(), "já está dentro dos parênteses");
    }

    #[test]
    fn a_sugestao_traz_assinatura_e_descricao() {
        let primeira = suggest_at("=PROCX", 6, 8).into_iter().next().unwrap();
        assert_eq!(primeira.name, "PROCX");
        assert!(primeira.signature.starts_with("PROCX(procurado; onde_procurar"), "{}", primeira.signature);
        assert!(primeira.help.unwrap().doc.contains("Busca moderna"));
    }

    #[test]
    fn identifica_a_chamada_e_o_argumento_sob_o_cursor() {
        let em = |t: &str| active_call(t, t.chars().count());
        assert_eq!(em("=PROCV(").unwrap().name, "PROCV");
        assert_eq!(em("=PROCV(A1;B1:C9;").unwrap().arg_index, 2);
        assert_eq!(em("=SOMA(SE(A1>1;").unwrap().name, "SE", "usa a chamada mais interna");
        assert_eq!(em("=SOMA(SE(A1>1;2);").unwrap().name, "SOMA", "fechou a interna");
        assert_eq!(em("=SOMA(\"a;b\";").unwrap().arg_index, 1, "\";\" dentro de texto não conta");
        assert_eq!(em("=A1+B1"), None);
    }

    #[test]
    fn referencia_por_clique_so_entra_em_posicao_valida() {
        assert!(accepts_reference("=", 1));
        assert!(accepts_reference("=SOMA(", 6));
        assert!(accepts_reference("=A1+", 4));
        assert!(accepts_reference("=SOMA(A1;", 9));
        assert!(!accepts_reference("=SOMA(A1", 8), "no meio de uma referência");
        assert!(!accepts_reference("=12", 3));
        assert!(!accepts_reference("=\"texto", 7));
        assert!(!accepts_reference("texto", 5));
    }

    #[test]
    fn encontra_as_referencias_para_destacar_na_grade() {
        let textos = |t: &str| references_in(t).into_iter().map(|r| r.0).collect::<Vec<_>>();
        assert_eq!(textos("=SOMA(A1:B3)+C4"), vec!["A1:B3", "C4"]);
        assert!(textos("=LOG10(2)").is_empty());
        assert_eq!(textos("=\"A1\"&B2"), vec!["B2"]);
    }

    #[test]
    fn token_antes_do_cursor() {
        assert_eq!(token_before("=SOMA(PRO", 9), Some(Token { text: "PRO".into(), start: 6, end: 9 }));
        assert_eq!(token_before("=SOMA(", 6), None);
    }

    #[test]
    fn cursor_alem_do_fim_conta_como_letra_como_no_typescript() {
        // O `IDENT.test(undefined)` do TypeScript casa (testa "undefined"):
        // conferido rodando o `assist.ts` — mesmos três resultados.
        assert_eq!(token_before("=MÉD", 99), Some(Token { text: "MÉD".into(), start: 1, end: 99 }));
        assert_eq!(token_before("=SOMA(", 10), Some(Token { text: String::new(), start: 6, end: 10 }));
        assert_eq!(token_before("", 3), Some(Token { text: String::new(), start: 0, end: 3 }));
        // Para a sugestão não muda nada: o prefixo é o mesmo.
        let nomes: Vec<_> = suggest_at("=SOM", 9, 8).into_iter().map(|s| s.name).collect();
        // Não é igualdade: cada função nova com prefixo "SOM" entraria aqui.
        assert_eq!(nomes[..3], ["SOMA", "SOMASE", "SOMASES"]);
        assert!(nomes.iter().any(|n| n == "SOMARPRODUTO"));
    }

    #[test]
    fn caixa_com_aba_e_travas_se_reescreve() {
        let t = "='Meu Plano'!$A1:B$2+Dados!C3";
        let caixas = reference_boxes(t);
        assert_eq!(caixas.len(), 2);
        assert_eq!(caixas[0].sheet.as_deref(), Some("Meu Plano"));
        assert_eq!(caixas[0].locks, Travas { c0: true, r0: false, c1: false, r1: true });
        assert_eq!(caixas[1].sheet.as_deref(), Some("Dados"));
        let novo = rewrite_box(t, caixas[0].start, caixas[0].end, caixas[0].locks, &Rect::new(4, 1, 6, 2));
        assert_eq!(novo, "='Meu Plano'!$B5:C$7+Dados!C3");
    }

    #[test]
    fn f4_alterna_referencia_sob_o_cursor() {
        let mut texto = "=SOMA(A1)+B2".to_string();
        let cursor = texto.find("A1").unwrap() + 1;
        for esperado in ["=SOMA($A$1)+B2", "=SOMA(A$1)+B2", "=SOMA($A1)+B2", "=SOMA(A1)+B2"] {
            texto = toggle_reference(&texto, cursor).unwrap();
            assert_eq!(texto, esperado);
        }
    }

    #[test]
    fn f4_preserva_texto_entre_aspas_e_o_resto_da_formula() {
        let texto = "=\"A1\"+B2";
        assert_eq!(toggle_reference(texto, 2), None);
        assert_eq!(toggle_reference(texto, 7).as_deref(), Some("=\"A1\"+$B$2"));
    }

    #[test]
    fn posicoes_de_referencia_preservam_unicode() {
        // O editor Iced indexa por caracteres; offsets de bytes não servem
        // quando há emoji ou caracteres acentuados antes da referência.
        let texto = "=SE(\"🚂\";A1;'Água'!$B2)";
        let caixas = reference_boxes(texto);
        assert_eq!(caixas.len(), 2);
        let letras: Vec<char> = texto.chars().collect();
        let primeira: String = letras[caixas[0].start..caixas[0].end].iter().collect();
        let segunda: String = letras[caixas[1].start..caixas[1].end].iter().collect();
        assert_eq!(primeira, "A1");
        assert_eq!(segunda, "$B2");
        assert_eq!(caixas[1].sheet.as_deref(), Some("Água"));
    }

}
