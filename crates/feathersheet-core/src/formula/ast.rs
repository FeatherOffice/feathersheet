//! A árvore de uma fórmula.
//!
//! Tradução de `src/core/ast.ts`.

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Num(f64),
    Str(String),
    Bool(bool),
    /// Literal de erro escrito na fórmula (`#N/A`), guardado como texto — é o
    /// que o TypeScript fazia, e há código de erro que o motor não conhece.
    Error(String),
    Ref {
        row: u32,
        col: u32,
        sheet: Option<String>,
    },
    Range {
        r0: u32,
        c0: u32,
        r1: u32,
        c1: u32,
        sheet: Option<String>,
    },
    /// Intervalo nomeado, ou qualquer nome que não seja função nem referência.
    Name(String),
    Call {
        name: String,
        args: Vec<Node>,
    },
    /// Argumento deixado em branco: `SEQUÊNCIA(3;;;0)`.
    Blank,
    /// Literal de matriz: `{1;2;3}` é uma coluna, `{1\2\3}` é uma linha.
    Array {
        rows: Vec<Vec<Node>>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Node>,
        right: Box<Node>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Node>,
    },
}

impl Node {
    /// O [`Self::walk`] que pode mudar os nós, na mesma ordem.
    pub fn walk_mut(&mut self, visit: &mut impl FnMut(&mut Node)) {
        visit(self);
        match self {
            Node::Binary { left, right, .. } => {
                left.walk_mut(visit);
                right.walk_mut(visit);
            }
            Node::Unary { operand, .. } => operand.walk_mut(visit),
            Node::Call { args, .. } => {
                for a in args {
                    a.walk_mut(visit);
                }
            }
            Node::Array { rows } => {
                for linha in rows {
                    for a in linha {
                        a.walk_mut(visit);
                    }
                }
            }
            _ => {}
        }
    }

    /// Percorre a árvore inteira, deste nó para baixo.
    pub fn walk(&self, visit: &mut impl FnMut(&Node)) {
        visit(self);
        match self {
            Node::Binary { left, right, .. } => {
                left.walk(visit);
                right.walk(visit);
            }
            Node::Unary { operand, .. } => operand.walk(visit),
            Node::Call { args, .. } => {
                for a in args {
                    a.walk(visit);
                }
            }
            Node::Array { rows } => {
                for linha in rows {
                    for a in linha {
                        a.walk(visit);
                    }
                }
            }
            _ => {}
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Concat,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl BinaryOp {
    pub fn from_text(texto: &str) -> Option<BinaryOp> {
        Some(match texto {
            "+" => BinaryOp::Add,
            "-" => BinaryOp::Sub,
            "*" => BinaryOp::Mul,
            "/" => BinaryOp::Div,
            "^" => BinaryOp::Pow,
            "&" => BinaryOp::Concat,
            "=" => BinaryOp::Eq,
            "<>" => BinaryOp::Ne,
            "<" => BinaryOp::Lt,
            "<=" => BinaryOp::Le,
            ">" => BinaryOp::Gt,
            ">=" => BinaryOp::Ge,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Pow => "^",
            BinaryOp::Concat => "&",
            BinaryOp::Eq => "=",
            BinaryOp::Ne => "<>",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
        }
    }

    /// Quanto maior, mais forte liga.
    pub fn precedence(self) -> u8 {
        match self {
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge => 1,
            BinaryOp::Concat => 2,
            BinaryOp::Add | BinaryOp::Sub => 3,
            BinaryOp::Mul | BinaryOp::Div => 4,
            BinaryOp::Pow => 5,
        }
    }

    pub fn right_assoc(self) -> bool {
        matches!(self, BinaryOp::Pow)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    /// `-A1`
    Neg,
    /// `+A1`
    Plus,
    /// `50%`
    Percent,
}

impl UnaryOp {
    pub fn as_str(self) -> &'static str {
        match self {
            UnaryOp::Neg => "-",
            UnaryOp::Plus => "+",
            UnaryOp::Percent => "%",
        }
    }
}

/// Fórmula que não compila.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    /// Onde está o caractere culpado dentro do corpo da fórmula, quando dá.
    ///
    /// É um índice **em caracteres**, não em bytes: quem mostra o erro na tela
    /// precisa contar do mesmo jeito, e o JavaScript conta em unidades UTF-16.
    /// Para o que aparece numa fórmula (acento, cifrão, letra grega) os dois
    /// batem; emoji na fórmula é o único caso em que não bateriam.
    pub at: Option<usize>,
}

impl ParseError {
    pub fn new(message: impl Into<String>, at: usize) -> ParseError {
        ParseError { message: message.into(), at: Some(at) }
    }

    pub fn solto(message: impl Into<String>) -> ParseError {
        ParseError { message: message.into(), at: None }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arvore_de_formula_cobre_ref_intervalo_matriz_e_operadores() {
        let arvore = Node::Call {
            name: "SOMA".into(),
            args: vec![
                Node::Ref { row: 1, col: 2, sheet: Some("Dados".into()) },
                Node::Range { r0: 0, c0: 0, r1: 5, c1: 2, sheet: None },
                Node::Array { rows: vec![
                    vec![Node::Num(1.0), Node::Bool(true)],
                    vec![Node::Unary { op: UnaryOp::Neg, operand: Box::new(Node::Num(2.0)) }],
                ] },
            ],
        };
        let mut visitados = Vec::new();
        arvore.walk(&mut |n| visitados.push(match n {
            Node::Call { .. } => "call",
            Node::Ref { .. } => "ref",
            Node::Range { .. } => "range",
            Node::Array { .. } => "array",
            Node::Num(_) => "num",
            Node::Bool(_) => "bool",
            Node::Unary { .. } => "unary",
            _ => "other",
        }));
        assert_eq!(visitados, vec!["call", "ref", "range", "array", "num", "bool", "unary", "num"]);
    }

    #[test]
    fn operadores_suportam_todas_as_formas_do_ast_typescript() {
        for (texto, op) in [
            ("+", BinaryOp::Add), ("-", BinaryOp::Sub), ("*", BinaryOp::Mul),
            ("/", BinaryOp::Div), ("^", BinaryOp::Pow), ("&", BinaryOp::Concat),
            ("=", BinaryOp::Eq), ("<>", BinaryOp::Ne), ("<", BinaryOp::Lt),
            ("<=", BinaryOp::Le), (">", BinaryOp::Gt), (">=", BinaryOp::Ge),
        ] {
            assert_eq!(BinaryOp::from_text(texto), Some(op), "{texto}");
            assert_eq!(op.as_str(), texto);
        }
        assert_eq!(BinaryOp::from_text("??"), None);
        for (op, txt) in [
            (UnaryOp::Neg, "-"), (UnaryOp::Plus, "+"), (UnaryOp::Percent, "%"),
        ] {
            assert_eq!(op.as_str(), txt);
        }
        assert!(BinaryOp::Pow.right_assoc());
        assert!(!BinaryOp::Add.right_assoc());
        assert!(BinaryOp::Mul.precedence() > BinaryOp::Add.precedence());
    }

    #[test]
    fn erro_de_parser_guarda_posicao_opcional() {
        let com_posicao = ParseError::new("símbolo inválido", 7);
        assert_eq!(com_posicao.at, Some(7));
        assert_eq!(com_posicao.to_string(), "símbolo inválido");
        let sem_posicao = ParseError::solto("fórmula incompleta");
        assert_eq!(sem_posicao.at, None);
    }
}
