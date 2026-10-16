//! Fórmulas → Avaliar fórmula: calcula a fórmula de uma célula um pedaço
//! por vez, como a janela do Excel. A cada passo, o próximo pedaço (o mais
//! interno à esquerda que já pode ser calculado) vira o seu valor.
//!
//! O valor de um pedaço vem sempre da árvore **original** dele, avaliada na
//! célula ([`Workbook::avaliar_em`]): o texto mostrado é só a apresentação.
//! Assim uma referência vazia aparece como 0 (igual ao Excel) sem estragar a
//! conta de quem a usa (`"a"&A1` continua dando "a").
//!
//! `SE` e `SEERRO` são preguiçosos: depois de calcular a condição (ou o
//! primeiro argumento), só o ramo escolhido continua a ser avaliado.
use std::collections::HashMap;

use crate::celulas::locale::get_locale_config;
use crate::celulas::number::to_js_string;
use crate::celulas::refs::{cell_name, quote_sheet};
use crate::celulas::values::{to_boolean, Value};
use crate::formula::ast::{Node, UnaryOp};
use crate::formula::parser::parse_formula;
use crate::planilha::workbook::Workbook;

type Caminho = Vec<usize>;

/// Funções cujo argumento é a própria referência, não o valor dela.
const DE_REFERENCIA: &[&str] = &[
    "LIN", "ROW", "COL", "COLUMN", "LINS", "ROWS", "COLS", "COLUMNS", "DESLOC", "OFFSET",
    "INDIRETO", "INDIRECT", "CÉL", "CELL", "ÉCÉL.VAZIA", "ISBLANK", "ÉREF", "ISREF", "ÁREAS", "AREAS",
    "FÓRMULATEXTO", "FORMULATEXT", "ÉFÓRMULA", "ISFORMULA",
];

fn eh_se(nome: &str) -> bool { matches!(nome.to_uppercase().as_str(), "SE" | "IF") }
fn eh_seerro(nome: &str) -> bool { matches!(nome.to_uppercase().as_str(), "SEERRO" | "IFERROR") }

fn filhos(n: &Node) -> Vec<&Node> {
    match n {
        Node::Call { args, .. } => args.iter().collect(),
        Node::Binary { left, right, .. } => vec![left, right],
        Node::Unary { operand, .. } => vec![operand],
        _ => Vec::new(),
    }
}

fn no_em<'a>(raiz: &'a Node, caminho: &[usize]) -> &'a Node {
    caminho.iter().fold(raiz, |n, &i| filhos(n)[i])
}

/// O estado da tela: a fórmula com os pedaços já calculados e onde está o
/// próximo (posição em caracteres de `texto`, para sublinhar).
#[derive(Debug, Clone, PartialEq)]
pub struct Estado {
    pub texto: String,
    pub proximo: Option<(usize, usize)>,
    pub concluida: bool,
    /// O próximo pedaço é uma referência a uma célula com fórmula: dá para "Entrar".
    pub pode_entrar: Option<(Option<String>, u32, u32)>,
}

#[derive(Debug, Clone)]
pub struct Avaliacao {
    pub aba: usize,
    pub linha: u32,
    pub coluna: u32,
    arvore: Node,
    calculados: HashMap<Caminho, Value>,
    /// SE/SEERRO já decidido: o índice do argumento que ficou.
    escolhas: HashMap<Caminho, usize>,
    /// O que cada passo fez, em texto ("A1 → 10").
    pub historico: Vec<String>,
}

impl Avaliacao {
    /// `None` quando a célula não tem fórmula (ou ela não analisa).
    pub fn nova(pasta: &Workbook, aba: usize, linha: u32, coluna: u32) -> Option<Self> {
        let bruto = pasta.aba(aba)?.sheet.get_raw(linha, coluna);
        let arvore = parse_formula(bruto.strip_prefix('=')?).ok()?;
        Some(Self { aba, linha, coluna, arvore, calculados: HashMap::new(), escolhas: HashMap::new(), historico: Vec::new() })
    }

    pub fn reiniciar(&mut self) {
        self.calculados.clear();
        self.escolhas.clear();
        self.historico.clear();
    }

    /// O próximo pedaço a calcular: o mais interno à esquerda.
    fn proximo(&self, n: &Node, c: &Caminho, pai_ref: bool, pasta: &Workbook) -> Option<Caminho> {
        if self.calculados.contains_key(c) { return None; }
        if let Some(&i) = self.escolhas.get(c) {
            let mut f = c.clone();
            f.push(i);
            return self.proximo(filhos(n)[i], &f, false, pasta).or_else(|| Some(c.clone()));
        }
        match n {
            Node::Ref { .. } => (!pai_ref).then(|| c.clone()),
            Node::Name(nome) => nome_de_celula(pasta, nome).then(|| c.clone()),
            Node::Call { name, args } => {
                let refs = DE_REFERENCIA.contains(&name.to_uppercase().as_str());
                // SE/SEERRO: primeiro o 1º argumento; depois a escolha do ramo.
                if (eh_se(name) || eh_seerro(name)) && !args.is_empty() {
                    let mut f = c.clone();
                    f.push(0);
                    return self.proximo(&args[0], &f, false, pasta).or_else(|| Some(c.clone()));
                }
                for (i, a) in args.iter().enumerate() {
                    let mut f = c.clone();
                    f.push(i);
                    if let Some(p) = self.proximo(a, &f, refs, pasta) { return Some(p); }
                }
                Some(c.clone())
            }
            Node::Binary { .. } | Node::Unary { .. } => {
                for (i, a) in filhos(n).into_iter().enumerate() {
                    let mut f = c.clone();
                    f.push(i);
                    if let Some(p) = self.proximo(a, &f, false, pasta) { return Some(p); }
                }
                Some(c.clone())
            }
            _ => None,
        }
    }

    fn valor_de(&self, n: &Node, c: &Caminho, pasta: &Workbook) -> Value {
        self.calculados.get(c).cloned().unwrap_or_else(|| pasta.avaliar_em(self.aba, self.linha, self.coluna, n))
    }

    /// Calcula o próximo pedaço. Devolve `false` quando já acabou.
    pub fn avancar(&mut self, pasta: &Workbook) -> bool {
        let Some(c) = self.proximo(&self.arvore, &Vec::new(), false, pasta) else { return false };
        let n = no_em(&self.arvore, &c).clone();
        if let Node::Call { name, args } = &n {
            if !self.escolhas.contains_key(&c) && !args.is_empty() && (eh_se(name) || eh_seerro(name)) {
                let primeiro = self.valor_de(&args[0], &[c.clone(), vec![0]].concat(), pasta);
                let escolha = if eh_se(name) {
                    match to_boolean(&primeiro) {
                        Ok(true) => Some(1),
                        Ok(false) if args.len() > 2 => Some(2),
                        _ => None, // sem 3º argumento (FALSO) ou condição com erro: calcula o SE inteiro
                    }
                } else if primeiro.is_error() { Some(1) } else { Some(0) };
                if let Some(i) = escolha.filter(|&i| i < args.len()) {
                    self.escolhas.insert(c.clone(), i);
                    self.historico.push(format!("{}: fica o {}º argumento", name.to_uppercase(), i + 1));
                    // Ramo que já é um valor: o SE termina neste mesmo passo.
                    let ramo = [c.clone(), vec![i]].concat();
                    if self.proximo(&args[i], &ramo, false, pasta).is_none() {
                        let valor = pasta.avaliar_em(self.aba, self.linha, self.coluna, &n);
                        self.calculados.insert(c, valor);
                    }
                    return true;
                }
            }
        }
        let antes = self.texto_do(&n, &c, pasta);
        let valor = pasta.avaliar_em(self.aba, self.linha, self.coluna, &n);
        self.historico.push(format!("{antes} → {}", literal(&valor)));
        self.calculados.insert(c, valor);
        true
    }

    pub fn concluida(&self, pasta: &Workbook) -> bool {
        self.proximo(&self.arvore, &Vec::new(), false, pasta).is_none()
    }

    fn texto_do(&self, n: &Node, c: &Caminho, pasta: &Workbook) -> String {
        let mut s = String::new();
        self.escrever(n, c, None, &mut s, &mut None, pasta);
        s
    }

    pub fn estado(&self, pasta: &Workbook) -> Estado {
        let alvo = self.proximo(&self.arvore, &Vec::new(), false, pasta);
        let mut texto = String::from("=");
        let mut marca = None;
        self.escrever(&self.arvore, &Vec::new(), alvo.as_ref(), &mut texto, &mut marca, pasta);
        let pode_entrar = alvo.as_ref().and_then(|c| match no_em(&self.arvore, c) {
            Node::Ref { row, col, sheet } => {
                let aba = match sheet {
                    Some(nome) => pasta.nomes().iter().position(|n| n.eq_ignore_ascii_case(nome))?,
                    None => self.aba,
                };
                pasta.aba(aba)?.sheet.is_formula(*row, *col).then(|| (sheet.clone(), *row, *col))
            }
            _ => None,
        });
        Estado { texto, proximo: marca, concluida: alvo.is_none(), pode_entrar }
    }

    /// Escreve a árvore como a pessoa lê (separadores da língua), trocando o
    /// que já foi calculado pelo valor e marcando onde começa e termina `alvo`.
    fn escrever(&self, n: &Node, c: &Caminho, alvo: Option<&Caminho>, s: &mut String,
        marca: &mut Option<(usize, usize)>, pasta: &Workbook) {
        let inicio = s.chars().count();
        if let Some(v) = self.calculados.get(c) {
            s.push_str(&literal(v));
        } else if let Some(&i) = self.escolhas.get(c) {
            let mut f = c.clone();
            f.push(i);
            self.escrever(filhos(n)[i], &f, alvo, s, marca, pasta);
        } else {
            let sep = get_locale_config().formula_arg_separator;
            match n {
                Node::Call { name, args } => {
                    s.push_str(name);
                    s.push('(');
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 { s.push(sep); }
                        let mut f = c.clone();
                        f.push(i);
                        self.escrever(a, &f, alvo, s, marca, pasta);
                    }
                    s.push(')');
                }
                Node::Binary { op, left, right } => {
                    for (i, lado) in [left, right].into_iter().enumerate() {
                        let parenteses = match lado.as_ref() {
                            Node::Binary { op: filho, .. } if !self.calculados.contains_key(&[c.clone(), vec![i]].concat()) =>
                                filho.precedence() < op.precedence() || (filho.precedence() == op.precedence() && i == 1 && !op.right_assoc()),
                            _ => false,
                        };
                        if i == 1 { s.push_str(op.as_str()); }
                        if parenteses { s.push('('); }
                        self.escrever(lado, &[c.clone(), vec![i]].concat(), alvo, s, marca, pasta);
                        if parenteses { s.push(')'); }
                    }
                }
                Node::Unary { op, operand } => {
                    let parenteses = matches!(operand.as_ref(), Node::Binary { .. })
                        && !self.calculados.contains_key(&[c.clone(), vec![0]].concat());
                    if *op != UnaryOp::Percent { s.push_str(op.as_str()); }
                    if parenteses { s.push('('); }
                    self.escrever(operand, &[c.clone(), vec![0]].concat(), alvo, s, marca, pasta);
                    if parenteses { s.push(')'); }
                    if *op == UnaryOp::Percent { s.push('%'); }
                }
                outro => s.push_str(&folha(outro, sep)),
            }
        }
        if alvo == Some(c) {
            *marca = Some((inicio, s.chars().count()));
        }
    }
}

/// Um nome definido que aponta para uma célula só (vira valor); intervalo fica como está.
fn nome_de_celula(pasta: &Workbook, nome: &str) -> bool {
    pasta.nomeados().iter().any(|n| n.nome.eq_ignore_ascii_case(nome) && n.area.r0 == n.area.r1 && n.area.c0 == n.area.c1)
}

fn numero(n: f64) -> String {
    let t = to_js_string(n);
    let decimal = get_locale_config().decimal_separator;
    if decimal == '.' { t } else { t.replace('.', &decimal.to_string()) }
}

/// Um valor escrito como literal de fórmula.
pub fn literal(v: &Value) -> String {
    match v {
        Value::Empty => "0".into(),
        Value::Number(n) => numero(*n),
        Value::Text(t) => format!("\"{}\"", t.replace('"', "\"\"")),
        Value::Bool(b) => if *b { "VERDADEIRO" } else { "FALSO" }.into(),
        Value::Error(e) => e.code.as_str().into(),
    }
}

fn folha(n: &Node, sep: char) -> String {
    let prefixo = |aba: &Option<String>| aba.as_deref().map(|a| format!("{}!", quote_sheet(a))).unwrap_or_default();
    match n {
        Node::Num(x) => numero(*x),
        Node::Str(t) => format!("\"{}\"", t.replace('"', "\"\"")),
        Node::Bool(b) => if *b { "VERDADEIRO" } else { "FALSO" }.into(),
        Node::Error(e) => e.clone(),
        Node::Ref { row, col, sheet } => format!("{}{}", prefixo(sheet), cell_name(*row, *col)),
        Node::Range { r0, c0, r1, c1, sheet } => format!("{}{}:{}", prefixo(sheet), cell_name(*r0, *c0), cell_name(*r1, *c1)),
        Node::Name(nome) => nome.clone(),
        Node::Blank => String::new(),
        Node::Array { rows } => {
            let (coluna, linha) = if sep == ';' { ('\\', ';') } else { (',', ';') };
            let corpo: Vec<String> = rows.iter()
                .map(|r| r.iter().map(|x| folha(x, sep)).collect::<Vec<_>>().join(&coluna.to_string()))
                .collect();
            format!("{{{}}}", corpo.join(&linha.to_string()))
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn pasta(celulas: &[(u32, u32, &str)]) -> Workbook {
        let mut w = Workbook::new();
        for &(r, c, v) in celulas {
            w.escrever_celula(0, r, c, v.into());
        }
        w
    }

    /// Avança até o fim, guardando o texto de cada estado.
    fn passos(w: &Workbook, r: u32, c: u32) -> Vec<String> {
        let mut a = Avaliacao::nova(w, 0, r, c).unwrap();
        let mut textos = vec![a.estado(w).texto];
        while a.avancar(w) {
            textos.push(a.estado(w).texto);
        }
        textos
    }

    #[test]
    fn calcula_do_mais_interno_para_fora() {
        let w = pasta(&[(0, 0, "10"), (1, 0, "20"), (2, 0, "30"), (0, 1, "2"), (0, 2, "=SOMA(A1:A3)*B1+1")]);
        assert_eq!(passos(&w, 0, 2), vec![
            "=SOMA(A1:A3)*B1+1",
            "=60*B1+1",
            "=60*2+1",
            "=120+1",
            "=121",
        ]);
    }

    #[test]
    fn marca_o_proximo_pedaco_e_respeita_parenteses() {
        let w = pasta(&[(0, 0, "1"), (0, 1, "2"), (0, 2, "=(A1+B1)*3")]);
        let a = Avaliacao::nova(&w, 0, 0, 2).unwrap();
        let e = a.estado(&w);
        assert_eq!(e.texto, "=(A1+B1)*3");
        let (i, f) = e.proximo.unwrap();
        assert_eq!(e.texto.chars().skip(i).take(f - i).collect::<String>(), "A1");
        assert_eq!(passos(&w, 0, 2).last().unwrap(), "=9");
        assert!(passos(&w, 0, 2).contains(&"=(1+2)*3".to_string()));
    }

    #[test]
    fn se_avalia_so_o_ramo_escolhido() {
        let w = pasta(&[(0, 0, "5"), (0, 1, "=SE(A1>3;\"alto\";1/0)")]);
        let t = passos(&w, 0, 1);
        assert_eq!(t, ["=SE(A1>3;\"alto\";1/0)", "=SE(5>3;\"alto\";1/0)", "=SE(VERDADEIRO;\"alto\";1/0)", "=\"alto\""]);
        assert!(!t.iter().any(|x| x.contains("#DIV/0!")), "o ramo do erro nunca foi calculado: {t:?}");
    }

    #[test]
    fn vazio_aparece_como_zero_sem_estragar_a_conta() {
        let w = pasta(&[(0, 1, "=\"a\"&A1")]);
        let t = passos(&w, 0, 1);
        assert_eq!(t[1], "=\"a\"&0");
        assert_eq!(t.last().unwrap(), "=\"a\"");
    }

    #[test]
    fn referencia_a_formula_permite_entrar() {
        let w = pasta(&[(0, 0, "2"), (0, 1, "=A1*10"), (0, 2, "=B1+LIN(B1)")]);
        let a = Avaliacao::nova(&w, 0, 0, 2).unwrap();
        assert_eq!(a.estado(&w).pode_entrar, Some((None, 0, 1)));
        let t = passos(&w, 0, 2);
        assert!(t.contains(&"=20+LIN(B1)".to_string()), "LIN recebe a referência, não o valor: {t:?}");
        assert_eq!(t.last().unwrap(), "=21");
        assert!(Avaliacao::nova(&w, 0, 0, 0).is_none(), "célula sem fórmula");
    }
}
