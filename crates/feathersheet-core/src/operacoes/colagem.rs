//! Operação do "Colar especial" do Excel: o valor colado é somado,
//! subtraído, multiplicado ou dividido com o que já está na célula.
//!
//! As regras seguem o Excel:
//! - número com número (vazio conta como 0) vira o resultado da conta;
//! - fórmula de qualquer lado vira fórmula: `=(B1*2)+5`, `=10-(A1+1)`;
//! - texto (dos dois lados) não entra na conta: a célula fica como estava.
use crate::celulas::{input::parse_literal, number::to_js_string, values::Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OperacaoColagem {
    #[default]
    Nenhuma,
    Somar,
    Subtrair,
    Multiplicar,
    Dividir,
}

impl OperacaoColagem {
    fn sinal(self) -> &'static str {
        match self {
            Self::Nenhuma => "",
            Self::Somar => "+",
            Self::Subtrair => "-",
            Self::Multiplicar => "*",
            Self::Dividir => "/",
        }
    }
}

/// Um lado da conta: número, fórmula (sem o `=`) ou algo que não entra nela.
enum Termo<'a> {
    Numero(f64),
    Formula(&'a str),
    Fora,
}

fn termo(raw: &str) -> Termo<'_> {
    if let Some(formula) = raw.strip_prefix('=') { return Termo::Formula(formula); }
    match parse_literal(raw).value {
        Value::Empty => Termo::Numero(0.0),
        Value::Number(n) => Termo::Numero(n),
        _ => Termo::Fora,
    }
}

/// Novo conteúdo da célula de destino depois da operação, ou `None` quando
/// a célula deve ficar como está (texto, lógico ou erro em algum dos lados).
/// `destino` e `origem` são o conteúdo cru (o que a pessoa digitou).
pub fn combinar(destino: &str, origem: &str, operacao: OperacaoColagem) -> Option<String> {
    if operacao == OperacaoColagem::Nenhuma { return Some(origem.to_owned()); }
    let sinal = operacao.sinal();
    match (termo(destino), termo(origem)) {
        (Termo::Fora, _) | (_, Termo::Fora) => None,
        (Termo::Numero(a), Termo::Numero(b)) => Some(match operacao {
            OperacaoColagem::Somar => to_js_string(a + b),
            OperacaoColagem::Subtrair => to_js_string(a - b),
            OperacaoColagem::Multiplicar => to_js_string(a * b),
            OperacaoColagem::Dividir if b == 0.0 => "#DIV/0!".into(),
            OperacaoColagem::Dividir => to_js_string(a / b),
            OperacaoColagem::Nenhuma => unreachable!(),
        }),
        (Termo::Formula(f), Termo::Numero(b)) => Some(format!("=({f}){sinal}{}", to_js_string(b))),
        (Termo::Numero(a), Termo::Formula(g)) => Some(format!("={}{sinal}({g})", to_js_string(a))),
        (Termo::Formula(f), Termo::Formula(g)) => Some(format!("=({f}){sinal}({g})")),
    }
}

#[cfg(test)]
mod testes {
    use super::{combinar, OperacaoColagem as Op};

    #[test]
    fn numeros_fazem_a_conta_e_vazio_vale_zero() {
        assert_eq!(combinar("10", "5", Op::Somar).as_deref(), Some("15"));
        assert_eq!(combinar("10", "5", Op::Subtrair).as_deref(), Some("5"));
        assert_eq!(combinar("10", "2,5", Op::Multiplicar).as_deref(), Some("25"));
        assert_eq!(combinar("", "5", Op::Somar).as_deref(), Some("5"));
        assert_eq!(combinar("7", "", Op::Multiplicar).as_deref(), Some("0"));
        assert_eq!(combinar("1", "0", Op::Dividir).as_deref(), Some("#DIV/0!"));
    }

    #[test]
    fn formula_de_qualquer_lado_vira_formula() {
        assert_eq!(combinar("=B1*2", "5", Op::Somar).as_deref(), Some("=(B1*2)+5"));
        assert_eq!(combinar("10", "=A1+1", Op::Subtrair).as_deref(), Some("=10-(A1+1)"));
        assert_eq!(combinar("=A1", "=B1", Op::Dividir).as_deref(), Some("=(A1)/(B1)"));
    }

    #[test]
    fn texto_nao_entra_na_conta_e_nenhuma_cola_a_origem() {
        assert_eq!(combinar("abc", "5", Op::Somar), None);
        assert_eq!(combinar("10", "abc", Op::Somar), None);
        assert_eq!(combinar("10", "abc", Op::Nenhuma).as_deref(), Some("abc"));
    }
}
