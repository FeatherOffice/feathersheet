//! Tradução da interface (menus, faixa, diálogos, janelas, barra de status).
//!
//! O texto em português escrito no código é a fonte e também a chave: em
//! pt-BR, [`tr`] devolve o próprio texto; em en-US, procura a chave em
//! `src/idiomas/interface.en-US.json` e, se ela faltar, cai no português.
//! O idioma é global ao processo e a tela troca na hora: as funções `view`
//! chamam `tr!` a cada quadro, então basta mudar o idioma (ver [`definir`]).
//!
//! Uso:
//! - `tr!("Salvar")` — texto fixo ou `&str` qualquer (o resultado vive o
//!   mesmo tanto que a entrada; quando traduzido, é `'static`).
//! - `trf!("Linha {} de {total}", n)` — mesmo formato do `format!`, conferido
//!   pelo compilador. A tradução repete os mesmos marcadores (`{}`, `{total}`,
//!   `{:.2}`...), em qualquer ordem; marcadores iguais mantêm a ordem relativa.
//!
//! As descrições das funções da planilha ficam em
//! `src/idiomas/funcoes.en-US.json`, lidas por [`ajuda_funcao`]; o motor
//! (`feathersheet-core`) continua só em português e sem dependência nova.
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

const JSON_INTERFACE_EN: &str = include_str!("../../../src/idiomas/interface.en-US.json");
const JSON_FUNCOES_EN: &str = include_str!("../../../src/idiomas/funcoes.en-US.json");

static INGLES: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
thread_local! {
    /// Nos testes (paralelos) o global fica sempre em pt-BR; um teste que
    /// queira ver a tela em inglês liga só a própria thread.
    static INGLES_THREAD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Chamado na abertura e ao trocar o idioma nas Configurações.
pub fn definir(idioma: &str) {
    if cfg!(test) { return; }
    INGLES.store(idioma.starts_with("en"), Ordering::Relaxed);
    // F12 e barra de baixo do feather-ui usam o mesmo dicionário.
    feather_ui::idioma::definir_tradutor(|pt| {
        if !em_ingles() { return None; }
        dicionario().get(pt).map(String::as_str)
    });
}

/// Só para testes: liga/desliga o inglês nesta thread.
#[cfg(test)]
pub fn definir_na_thread(idioma: &str) {
    INGLES_THREAD.with(|c| c.set(idioma.starts_with("en")));
}

/// A interface está em inglês agora?
pub fn em_ingles() -> bool {
    #[cfg(test)]
    { INGLES_THREAD.with(|c| c.get()) }
    #[cfg(not(test))]
    { INGLES.load(Ordering::Relaxed) }
}

/// Dicionário português → inglês da interface. Chaves que começam com `//`
/// são comentários do arquivo e não entram.
pub fn dicionario() -> &'static HashMap<String, String> {
    static D: OnceLock<HashMap<String, String>> = OnceLock::new();
    D.get_or_init(|| {
        let mapa: HashMap<String, String> = serde_json::from_str(JSON_INTERFACE_EN)
            .expect("interface.en-US.json inválido");
        mapa.into_iter().filter(|(k, _)| !k.starts_with("//")).collect()
    })
}

/// Traduz um texto da interface para o idioma atual; sem tradução, devolve
/// o próprio texto em português.
pub fn tr(pt: &str) -> &str {
    if !em_ingles() { return pt; }
    match dicionario().get(pt) {
        Some(en) => en.as_str(),
        None => pt,
    }
}

/// O texto em português de um texto já traduzido (o inverso de [`tr`]);
/// sem correspondência, devolve o próprio texto. Serve a quem decide algo
/// pelo rótulo exibido (ícone do item de menu, por exemplo).
pub fn original(mostrado: &str) -> &str {
    if !em_ingles() { return mostrado; }
    static INVERSO: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    let inv = INVERSO.get_or_init(|| {
        let mut m = HashMap::new();
        for (pt, en) in dicionario() { m.entry(en.as_str()).or_insert(pt.as_str()); }
        m
    });
    inv.get(mostrado).copied().unwrap_or(mostrado)
}

/// Versão traduzida de uma lista fixa de opções (listas suspensas pedem
/// `&'static [&'static str]`). Cada lista é traduzida uma vez e guardada.
pub fn lista(pt: &'static [&'static str]) -> &'static [&'static str] {
    if !em_ingles() { return pt; }
    type Cache = std::sync::Mutex<Vec<((usize, usize), &'static [&'static str])>>;
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let chave = (pt.as_ptr() as usize, pt.len());
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, l)) = cache.iter().find(|(k, _)| *k == chave) { return l; }
    let traduzida: &'static [&'static str] = Box::leak(pt.iter().map(|s| tr(s)).collect::<Vec<_>>().into_boxed_slice());
    cache.push((chave, traduzida));
    traduzida
}

/// Volta da tela para a chave: dentre `chaves` (em português), a que,
/// traduzida, é o texto `mostrado` (listas suspensas devolvem o texto exibido).
pub fn chave_de<'a>(chaves: &[&'a str], mostrado: &str) -> Option<&'a str> {
    chaves.iter().copied().find(|pt| *pt == mostrado || tr(pt) == mostrado)
}

/// Um pedaço do modelo de formato: texto literal ou marcador `{...}`.
#[derive(Debug, Clone, PartialEq)]
enum Peca { Texto(String), Marcador(String) }

/// Divide o modelo do `format!` em texto e marcadores (`{{`/`}}` viram chave).
fn pecas(modelo: &str) -> Vec<Peca> {
    let mut saida = Vec::new();
    let mut texto = String::new();
    let mut it = modelo.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '{' if it.peek() == Some(&'{') => { it.next(); texto.push('{'); }
            '}' if it.peek() == Some(&'}') => { it.next(); texto.push('}'); }
            '{' => {
                if !texto.is_empty() { saida.push(Peca::Texto(std::mem::take(&mut texto))); }
                let mut m = String::from("{");
                for d in it.by_ref() { m.push(d); if d == '}' { break; } }
                saida.push(Peca::Marcador(m));
            }
            _ => texto.push(c),
        }
    }
    if !texto.is_empty() { saida.push(Peca::Texto(texto)); }
    saida
}

/// Marcadores do modelo, na ordem (usado nos testes de consistência).
#[cfg_attr(not(test), allow(dead_code))]
pub fn marcadores(modelo: &str) -> Vec<String> {
    pecas(modelo).into_iter().filter_map(|p| match p {
        Peca::Marcador(m) => Some(m), Peca::Texto(_) => None,
    }).collect()
}

/// Recupera o valor de cada marcador comparando o texto já formatado em
/// português com o modelo. `None` se o texto não casar com o modelo.
fn valores(modelo: &[Peca], formatado: &str) -> Option<Vec<String>> {
    let mut resto = formatado;
    let mut valores = Vec::new();
    let mut i = 0;
    while i < modelo.len() {
        match &modelo[i] {
            Peca::Texto(t) => { resto = resto.strip_prefix(t.as_str())?; i += 1; }
            Peca::Marcador(_) => {
                // O valor vai até o próximo texto literal (ou até o fim).
                match modelo.get(i + 1) {
                    Some(Peca::Texto(prox)) => {
                        let ultimo = i + 2 >= modelo.len();
                        let fim = if ultimo { resto.rfind(prox.as_str())? } else { resto.find(prox.as_str())? };
                        if ultimo && fim + prox.len() != resto.len() { return None; }
                        valores.push(resto[..fim].to_owned());
                        resto = &resto[fim..];
                    }
                    // Dois marcadores colados: não dá para separar com segurança.
                    Some(Peca::Marcador(_)) => return None,
                    None => { valores.push(resto.to_owned()); resto = ""; }
                }
                i += 1;
            }
        }
    }
    resto.is_empty().then_some(valores)
}

/// Monta a versão traduzida de um texto formatado: `modelo_pt` é o literal
/// do `format!`, `formatado` é o resultado em português.
pub fn traduzir_formatado(modelo_pt: &str, formatado: String) -> String {
    if !em_ingles() { return formatado; }
    let Some(modelo_en) = dicionario().get(modelo_pt) else { return formatado; };
    let pt = pecas(modelo_pt);
    let Some(vals) = valores(&pt, &formatado) else { return formatado; };
    let marc_pt: Vec<&str> = pt.iter().filter_map(|p| match p {
        Peca::Marcador(m) => Some(m.as_str()), Peca::Texto(_) => None,
    }).collect();
    let mut usados = vec![false; marc_pt.len()];
    let mut saida = String::with_capacity(formatado.len() + 16);
    for p in pecas(modelo_en) {
        match p {
            Peca::Texto(t) => saida.push_str(&t),
            Peca::Marcador(m) => {
                // Primeiro marcador ainda não usado com o mesmo texto.
                let achado = marc_pt.iter().enumerate()
                    .position(|(j, x)| !usados[j] && *x == m)
                    .or_else(|| marc_pt.iter().position(|x| *x == m));
                match achado {
                    Some(j) => { usados[j] = true; saida.push_str(&vals[j]); }
                    None => return formatado,
                }
            }
        }
    }
    saida
}

/// `tr!(texto)`: traduz para o idioma atual da interface.
#[macro_export]
macro_rules! tr {
    ($e:expr) => { $crate::idioma::tr($e) };
}

/// `trf!(modelo, args...)`: como `format!`, traduzindo o modelo.
#[macro_export]
macro_rules! trf {
    ($modelo:literal $($resto:tt)*) => {
        $crate::idioma::traduzir_formatado($modelo, format!($modelo $($resto)*))
    };
}

/// Descrição e argumentos de uma função da planilha no idioma da interface.
/// O motor guarda só o português; em inglês, a descrição vem de
/// `funcoes.en-US.json` (por nome canônico) e cada argumento, do mapa de
/// argumentos (preservando `[ ]` de opcional e números no fim).
pub fn ajuda_funcao(nome: &str) -> Option<(Vec<String>, String)> {
    ajuda_funcao_em(nome, if em_ingles() { "en-US" } else { "pt-BR" })
}

/// Como [`ajuda_funcao`], com o idioma dado (a Ajuda F1 monta um catálogo por idioma).
pub fn ajuda_funcao_em(nome: &str, idioma: &str) -> Option<(Vec<String>, String)> {
    let ajuda = feathersheet_core::funcoes::function_help::help_for(nome)?;
    if !idioma.starts_with("en") {
        return Some((ajuda.args.iter().map(|a| (*a).to_owned()).collect(), ajuda.doc.to_owned()));
    }
    let f = funcoes_en();
    let doc = f.descricoes.get(nome).cloned().unwrap_or_else(|| ajuda.doc.to_owned());
    let args = ajuda.args.iter().map(|a| argumento_en(a)).collect();
    Some((args, doc))
}

/// Argumentos e descrição de uma ajuda já obtida do motor (por exemplo, a
/// do assistente de fórmula), no idioma da interface.
pub fn ajuda_traduzida(h: &feathersheet_core::funcoes::function_help::FunctionHelp) -> (Vec<String>, String) {
    if !em_ingles() {
        return (h.args.iter().map(|a| (*a).to_owned()).collect(), h.doc.to_owned());
    }
    let nome = feathersheet_core::funcoes::function_help::all().iter()
        .find(|(_, x)| std::ptr::eq(x.doc, h.doc) || x.doc == h.doc).map(|(n, _)| *n);
    let doc = nome.and_then(|n| funcoes_en().descricoes.get(n).cloned())
        .unwrap_or_else(|| h.doc.to_owned());
    (h.args.iter().map(|a| argumento_en(a)).collect(), doc)
}

/// Assinatura `NOME(arg1; arg2)` no idioma da interface (em inglês, com vírgula).
pub fn assinatura(nome: &str, h: &feathersheet_core::funcoes::function_help::FunctionHelp) -> String {
    if !em_ingles() { return feathersheet_core::funcoes::function_help::signature(nome, h); }
    format!("{}({})", nome.to_uppercase(), ajuda_traduzida(h).0.join(", "))
}

/// Nome de categoria de funções no idioma da interface.
pub fn categoria_funcao(pt: &str) -> &str {
    if !em_ingles() { return pt; }
    funcoes_en().categorias.get(pt).map(String::as_str).unwrap_or(pt)
}

#[derive(serde::Deserialize, Default)]
pub struct FuncoesEn {
    #[serde(default)]
    pub categorias: HashMap<String, String>,
    #[serde(default)]
    pub argumentos: HashMap<String, String>,
    #[serde(default)]
    pub descricoes: HashMap<String, String>,
}

pub fn funcoes_en() -> &'static FuncoesEn {
    static F: OnceLock<FuncoesEn> = OnceLock::new();
    F.get_or_init(|| serde_json::from_str(JSON_FUNCOES_EN).expect("funcoes.en-US.json inválido"))
}

/// Traduz um nome de argumento: `[núm2]` → `[number2]`; sem tradução, o próprio.
pub fn argumento_en(pt: &str) -> String {
    argumento_en_opt(pt).unwrap_or_else(|| pt.to_owned())
}

/// Tradução de um argumento, se o mapa a cobrir (direta, entre colchetes de
/// opcional, ou com número no fim: `valor2` → `value2`).
pub fn argumento_en_opt(pt: &str) -> Option<String> {
    let mapa = &funcoes_en().argumentos;
    if let Some(en) = mapa.get(pt) { return Some(en.clone()); }
    let (abre, meio, fecha) = match pt.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        Some(m) => ("[", m, "]"),
        None => ("", pt, ""),
    };
    if let Some(en) = mapa.get(meio) { return Some(format!("{abre}{en}{fecha}")); }
    let base = meio.trim_end_matches(|c: char| c.is_ascii_digit());
    let num = &meio[base.len()..];
    mapa.get(base).map(|en| format!("{abre}{en}{num}{fecha}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portugues_devolve_o_proprio_texto() {
        definir_na_thread("pt-BR");
        assert_eq!(tr("Salvar"), "Salvar");
        assert_eq!(trf!("Linha {} de {}", 3, 9), "Linha 3 de 9");
    }

    #[test]
    fn ingles_traduz_e_cai_no_portugues_sem_chave() {
        definir_na_thread("en-US");
        assert_eq!(tr("Salvar"), "Save");
        assert_eq!(tr("texto que não existe no dicionário"), "texto que não existe no dicionário");
        definir_na_thread("pt-BR");
    }

    #[test]
    fn formatado_recupera_os_valores_dos_marcadores() {
        let modelo = pecas("{} de {total} ({:.1}%)");
        let v = valores(&modelo, "3 de 10 (30.0%)").unwrap();
        assert_eq!(v, vec!["3", "10", "30.0"]);
        assert!(valores(&pecas("A{}B"), "XYZ").is_none());
    }

    #[test]
    fn dicionario_mantem_os_mesmos_marcadores() {
        for (pt, en) in dicionario() {
            let mut a = marcadores(pt); a.sort();
            let mut b = marcadores(en); b.sort();
            assert_eq!(a, b, "marcadores diferentes em {pt:?} → {en:?}");
        }
    }

    #[test]
    fn toda_funcao_tem_descricao_e_argumentos_em_ingles() {
        use feathersheet_core::funcoes::function_help::all;
        let f = funcoes_en();
        let mut faltam = Vec::new();
        for (nome, ajuda) in all() {
            if !f.descricoes.contains_key(*nome) { faltam.push(format!("descrição de {nome}")); }
            for a in ajuda.args {
                if *a != "…" && argumento_en_opt(a).is_none() {
                    faltam.push(format!("argumento {a} de {nome}"));
                }
            }
        }
        assert!(faltam.is_empty(), "faltam traduções: {faltam:#?}");
    }

    #[test]
    fn categorias_de_funcoes_tem_ingles() {
        for (c, _) in feathersheet_core::funcoes::function_help::CATEGORIES {
            assert!(funcoes_en().categorias.contains_key(*c), "categoria {c}");
        }
    }

    /// Lê o literal de string Rust que começa em `ini` (logo após a aspa);
    /// devolve o texto já sem escapes e onde ele termina.
    fn literal(fonte: &str, ini: usize) -> Option<(String, usize)> {
        let b = fonte.as_bytes();
        let mut texto = String::new();
        let mut i = ini;
        while i < b.len() {
            match b[i] {
                b'"' => return Some((texto, i + 1)),
                b'\\' => {
                    let prox = fonte[i + 1..].chars().next()?;
                    i += 1 + prox.len_utf8();
                    match prox {
                        'n' => texto.push('\n'),
                        't' => texto.push('\t'),
                        '0' => texto.push('\0'),
                        'u' => {
                            let fim = fonte[i..].find('}')? + i;
                            let cod = u32::from_str_radix(&fonte[i + 1..fim], 16).ok()?;
                            texto.push(char::from_u32(cod)?);
                            i = fim + 1;
                        }
                        '\n' => {
                            while i < b.len() && (b[i] as char).is_whitespace() { i += 1; }
                        }
                        c => texto.push(c),
                    }
                }
                _ => {
                    let c = fonte[i..].chars().next()?;
                    texto.push(c);
                    i += c.len_utf8();
                }
            }
        }
        None
    }

    /// Todo `tr!("…")` e `trf!("…")` da interface (fora deste arquivo) com
    /// um texto fixo, e onde ele aparece.
    fn chaves_usadas() -> Vec<(String, String)> {
        fn varrer(dir: &std::path::Path, saida: &mut Vec<(String, String)>) {
            let Ok(entradas) = std::fs::read_dir(dir) else { return };
            for e in entradas.flatten() {
                let p = e.path();
                if p.is_dir() { varrer(&p, saida); continue; }
                if p.extension().and_then(|x| x.to_str()) != Some("rs") || p.ends_with("idioma.rs") { continue; }
                let Ok(fonte) = std::fs::read_to_string(&p) else { continue };
                for marca in ["tr!(", "trf!("] {
                    let mut de = 0;
                    while let Some(pos) = fonte[de..].find(marca) {
                        let ini = de + pos;
                        de = ini + marca.len();
                        // `include_str!(` e afins não contam.
                        if fonte[..ini].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_') { continue; }
                        let resto = &fonte[de..];
                        let pulo = resto.len() - resto.trim_start().len();
                        if !resto[pulo..].starts_with('"') { continue; }
                        if let Some((texto, _)) = literal(&fonte, de + pulo + 1) {
                            let linha = fonte[..ini].matches('\n').count() + 1;
                            saida.push((texto, format!("{}:{linha}", p.display())));
                        }
                    }
                }
            }
        }
        let mut saida = Vec::new();
        varrer(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut saida);
        saida
    }

    /// O teste pedido: com en-US, toda chave usada na interface tem tradução.
    #[test]
    fn toda_chave_usada_na_interface_tem_traducao() {
        let usadas = chaves_usadas();
        assert!(usadas.len() > 2000, "a varredura achou só {} chaves", usadas.len());
        let faltam: Vec<_> = usadas.iter()
            .filter(|(k, _)| !dicionario().contains_key(k))
            .map(|(k, onde)| format!("{onde}: {k:?}")).collect();
        assert!(faltam.is_empty(), "{} chave(s) sem tradução:\n{}", faltam.len(), faltam.join("\n"));
    }

    /// Com en-US, toda chave usada sai diferente do português, salvo as que
    /// são iguais nas duas línguas (nomes próprios, siglas, unidades).
    #[test]
    fn com_ingles_nenhuma_chave_usada_fica_em_portugues() {
        definir_na_thread("en-US");
        let iguais: Vec<_> = chaves_usadas().into_iter()
            .filter(|(k, _)| tr(k) == k && k.chars().any(|c| c.is_alphabetic() && !c.is_ascii()))
            .map(|(k, onde)| format!("{onde}: {k:?}")).collect();
        definir_na_thread("pt-BR");
        assert!(iguais.is_empty(), "continuam em português:\n{}", iguais.join("\n"));
    }

    #[test]
    fn faixa_e_menus_ja_tem_traducao_no_dicionario() {
        // A faixa e o menu Editar ainda escrevem o texto direto (estão sendo
        // refeitos por outra pessoa); o dicionário já cobre seus rótulos.
        for pt in ["Área de transferência", "Pincel", "Classificar e filtrar", "Tabela dinâmica",
            "Gráficos", "Mesclar e centralizar", "Definir área de impressão", "Configurar Página",
            "Boas-vindas ao FeatherSheet", "Ferramentas de dados", "Proteger intervalo"] {
            assert!(dicionario().contains_key(pt), "{pt}");
        }
    }
}
