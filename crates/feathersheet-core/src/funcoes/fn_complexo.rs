//! Números complexos em Rust: COMPLEXO, IMABS, IMAGINÁRIO, IMARG, IMCONJ, IMCOS, IMCOSH, IMCOT, IMCOSEC, IMCOSECH, IMDIV, IMEXP, IMLN, IMLOG10, IMLOG2, IMPOT, IMPROD, IMREAL, IMSEC, IMSECH, IMSENO, IMSENH, IMRAIZ, IMSUBTR, IMSOMA, IMTAN.
//!
//! Como no Excel, o número complexo mora numa célula como **texto**:
//! `"3+4i"`, `"-2j"`, `"i"`, `"1,5"`. O sufixo é `i` ou `j` (minúsculo) e a
//! resposta usa o mesmo sufixo da entrada; misturar os dois numa conta dá
//! #VALOR!. Texto que não é complexo dá #NÚM!. A saída escreve até 15
//! algarismos significativos, com vírgula decimal, como o resto da planilha.

use crate::celulas::values::{Computed, Value};
use crate::formula::ast::Node;
use crate::formula::evaluator::Evaluator;
use crate::funcoes::fn_helpers::{
    arity, concluir, e_div0, e_num, e_valor, numero, por_elemento, texto, Entradas, Res,
};

/// Um número complexo `re + im·i`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Cx {
    pub re: f64,
    pub im: f64,
}

impl Cx {
    pub(crate) const fn new(re: f64, im: f64) -> Cx {
        Cx { re, im }
    }

    fn abs(self) -> f64 {
        (self.re * self.re + self.im * self.im).sqrt()
    }

    fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }

    fn eh_zero(self) -> bool {
        self.re == 0.0 && self.im == 0.0
    }

    fn soma(self, o: Cx) -> Cx {
        Cx::new(self.re + o.re, self.im + o.im)
    }

    fn menos(self, o: Cx) -> Cx {
        Cx::new(self.re - o.re, self.im - o.im)
    }

    fn vezes(self, o: Cx) -> Cx {
        Cx::new(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
    }

    /// Divisão; divisor zero dá #NÚM!, como no IMDIV do Excel.
    fn dividido(self, o: Cx) -> Res<Cx> {
        let d = o.re * o.re + o.im * o.im;
        if d == 0.0 {
            return Err(e_num());
        }
        Ok(Cx::new(
            (self.re * o.re + self.im * o.im) / d,
            (self.im * o.re - self.re * o.im) / d,
        ))
    }

    fn inverso(self) -> Res<Cx> {
        Cx::new(1.0, 0.0).dividido(self)
    }

    fn seno(self) -> Cx {
        Cx::new(self.re.sin() * self.im.cosh(), self.re.cos() * self.im.sinh())
    }

    fn cosseno(self) -> Cx {
        Cx::new(self.re.cos() * self.im.cosh(), -self.re.sin() * self.im.sinh())
    }

    fn seno_h(self) -> Cx {
        Cx::new(self.re.sinh() * self.im.cos(), self.re.cosh() * self.im.sin())
    }

    fn cosseno_h(self) -> Cx {
        Cx::new(self.re.cosh() * self.im.cos(), self.re.sinh() * self.im.sin())
    }

    fn exp(self) -> Cx {
        let m = self.re.exp();
        Cx::new(m * self.im.cos(), m * self.im.sin())
    }

    /// Logaritmo natural (ramo principal); zero dá #NÚM!.
    fn ln(self) -> Res<Cx> {
        if self.eh_zero() {
            return Err(e_num());
        }
        Ok(Cx::new(self.abs().ln(), self.arg()))
    }

    /// Potência pela forma polar, `r^n·(cos nθ + i·sen nθ)`, a definição da
    /// documentação do IMPOT.
    fn potencia(self, n: f64) -> Res<Cx> {
        if self.eh_zero() {
            return if n > 0.0 { Ok(Cx::new(0.0, 0.0)) } else { Err(e_num()) };
        }
        let r = self.abs().powf(n);
        let t = self.arg() * n;
        Ok(Cx::new(r * t.cos(), r * t.sin()))
    }

    /// Raiz quadrada pela forma polar, `√r·(cos θ/2 + i·sen θ/2)`.
    fn raiz(self) -> Cx {
        let r = self.abs().sqrt();
        let t = self.arg() / 2.0;
        Cx::new(r * t.cos(), r * t.sin())
    }
}

// ---- texto ↔ complexo ----

/// Número real escrito na parte de um complexo: sinal opcional, algarismos
/// com no máximo um separador decimal (vírgula ou ponto) e expoente opcional.
fn real_do_texto(s: &str) -> Option<f64> {
    let b = s.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let mut algarismos = 0;
    let mut separador = false;
    while i < b.len() {
        match b[i] {
            b'0'..=b'9' => algarismos += 1,
            b'.' | b',' if !separador => separador = true,
            _ => break,
        }
        i += 1;
    }
    if algarismos == 0 {
        return None;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        let inicio = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == inicio {
            return None;
        }
    }
    if i != b.len() {
        return None;
    }
    s.replace(',', ".").parse::<f64>().ok().filter(|x| x.is_finite())
}

/// Coeficiente da parte imaginária: vazio ou só sinal valem ±1.
fn coeficiente(s: &str) -> Option<f64> {
    match s {
        "" | "+" => Some(1.0),
        "-" => Some(-1.0),
        _ => real_do_texto(s),
    }
}

/// Lê `"a+bi"`, `"a-bj"`, `"bi"`, `"i"`, `"a"`. Devolve o número e o sufixo
/// (`None` quando não há parte imaginária escrita).
pub(crate) fn complexo_do_texto(s: &str) -> Res<(Cx, Option<char>)> {
    if s.is_empty() {
        return Ok((Cx::new(0.0, 0.0), None));
    }
    let sufixo = match s.chars().last() {
        Some(c @ ('i' | 'j')) => c,
        _ => {
            let re = real_do_texto(s).ok_or_else(e_num)?;
            return Ok((Cx::new(re, 0.0), None));
        }
    };
    let corpo = &s[..s.len() - 1];
    // O sinal que separa as partes é o último `+`/`-` que não está no
    // começo nem logo depois de um `e` de expoente.
    let b = corpo.as_bytes();
    let corte = (1..b.len())
        .rev()
        .find(|&k| (b[k] == b'+' || b[k] == b'-') && !matches!(b[k - 1], b'e' | b'E'));
    let (re, im) = match corte {
        Some(k) => (real_do_texto(&corpo[..k]).ok_or_else(e_num)?, coeficiente(&corpo[k..])),
        None => (0.0, coeficiente(corpo)),
    };
    let im = im.ok_or_else(e_num)?;
    Ok((Cx::new(re, im), Some(sufixo)))
}

/// Um valor de célula como complexo: número é a parte real, texto é lido,
/// lógico dá #VALOR!, erro passa adiante.
fn complexo_de(v: &Value) -> Res<(Cx, Option<char>)> {
    match v {
        Value::Number(n) => Ok((Cx::new(*n, 0.0), None)),
        Value::Text(t) => complexo_do_texto(t),
        Value::Empty => Ok((Cx::new(0.0, 0.0), None)),
        Value::Bool(_) => Err(e_valor()),
        Value::Error(e) => Err(e.clone()),
    }
}

/// Junta dois sufixos: o escrito vence o ausente; `i` com `j` é #VALOR!.
fn junta_sufixo(a: Option<char>, b: Option<char>) -> Res<Option<char>> {
    match (a, b) {
        (Some(x), Some(y)) if x != y => Err(e_valor()),
        (Some(x), _) | (None, Some(x)) => Ok(Some(x)),
        (None, None) => Ok(None),
    }
}

/// Um número com até 15 algarismos significativos e vírgula decimal. Como
/// no texto dos números do resto da planilha, abaixo de 0,000001 ou a
/// partir de 1E+21 vai em notação científica (`1,5E-20`).
pub(crate) fn real_em_texto(x: f64) -> String {
    if x == 0.0 {
        return "0".to_string();
    }
    let cientifico = format!("{:.14e}", x);
    let (mantissa, expoente) = cientifico.split_once('e').unwrap_or((&cientifico, "0"));
    let expoente: i32 = expoente.parse().unwrap_or(0);
    let negativo = mantissa.starts_with('-');
    let algarismos: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let algarismos = algarismos.trim_end_matches('0');
    let algarismos = if algarismos.is_empty() { "0" } else { algarismos };
    let mut s = String::new();
    if negativo {
        s.push('-');
    }
    if !(-6..21).contains(&expoente) {
        s.push_str(&algarismos[..1]);
        if algarismos.len() > 1 {
            s.push(',');
            s.push_str(&algarismos[1..]);
        }
        s.push_str(&format!("E{}{}", if expoente < 0 { '-' } else { '+' }, expoente.abs()));
    } else if expoente < 0 {
        s.push_str("0,");
        for _ in 0..(-expoente - 1) {
            s.push('0');
        }
        s.push_str(algarismos);
    } else {
        let inteiro = expoente as usize + 1;
        if algarismos.len() <= inteiro {
            s.push_str(algarismos);
            for _ in algarismos.len()..inteiro {
                s.push('0');
            }
        } else {
            s.push_str(&algarismos[..inteiro]);
            s.push(',');
            s.push_str(&algarismos[inteiro..]);
        }
    }
    s
}

/// O complexo como texto: `"3+4i"`, `"-i"`, `"2,5"`, `"0"`.
pub(crate) fn complexo_em_texto(z: Cx, sufixo: char) -> Res<Value> {
    if !z.re.is_finite() || !z.im.is_finite() {
        return Err(e_num());
    }
    // Arredonda a 15 algarismos antes de decidir se a parte é zero.
    let re = real_em_texto(z.re);
    let im = real_em_texto(z.im);
    if im == "0" {
        return texto(re);
    }
    let coef = match im.as_str() {
        "1" => String::new(),
        "-1" => "-".to_string(),
        outro => outro.to_string(),
    };
    if re == "0" {
        return texto(format!("{coef}{sufixo}"));
    }
    let sinal = if im.starts_with('-') { "" } else { "+" };
    texto(format!("{re}{sinal}{coef}{sufixo}"))
}

// ---- moldes das funções ----

/// Função de um complexo que devolve complexo.
fn unaria(args: &[Node], ev: &Evaluator<'_>, conta: fn(Cx) -> Res<Cx>) -> Computed {
    por_elemento(args, ev, 1, 1, |e| {
        let (z, s) = complexo_de(e.valor(0))?;
        complexo_em_texto(conta(z)?, s.unwrap_or('i'))
    })
}

/// Função de um complexo que devolve número real.
fn para_real(args: &[Node], ev: &Evaluator<'_>, conta: fn(Cx) -> Res<f64>) -> Computed {
    por_elemento(args, ev, 1, 1, |e| {
        let (z, _) = complexo_de(e.valor(0))?;
        numero(conta(z)?)
    })
}

/// Função de dois complexos.
fn binaria(args: &[Node], ev: &Evaluator<'_>, conta: fn(Cx, Cx) -> Res<Cx>) -> Computed {
    por_elemento(args, ev, 2, 2, |e| {
        let (a, sa) = complexo_de(e.valor(0))?;
        let (b, sb) = complexo_de(e.valor(1))?;
        let s = junta_sufixo(sa, sb)?;
        complexo_em_texto(conta(a, b)?, s.unwrap_or('i'))
    })
}

/// IMSOMA/IMPROD: todos os complexos de todos os argumentos (intervalos
/// inclusive, células vazias puladas) combinados um a um.
fn acumula(args: &[Node], ev: &Evaluator<'_>, inicio: Cx, junta: fn(Cx, Cx) -> Cx) -> Computed {
    if let Some(e) = arity(args, 1, 255) {
        return Computed::Scalar(Value::Error(e));
    }
    concluir((|| {
        let mut total = inicio;
        let mut sufixo = None;
        for a in args {
            // Argumento em branco (`IMPROD("1+i";;"2")`) vale zero.
            if matches!(a, Node::Blank) {
                total = junta(total, Cx::new(0.0, 0.0));
                continue;
            }
            let mut falha = None;
            let erro = ev.for_each_value(a, &mut |v, _| {
                if falha.is_some() || v.is_empty() {
                    return;
                }
                match complexo_de(v).and_then(|(z, s)| Ok((z, junta_sufixo(sufixo, s)?))) {
                    Ok((z, s)) => {
                        total = junta(total, z);
                        sufixo = s;
                    }
                    Err(e) => falha = Some(e),
                }
            });
            if let Some(e) = erro.or(falha) {
                return Err(e);
            }
        }
        complexo_em_texto(total, sufixo.unwrap_or('i'))
    })())
}

// ---- as funções ----

/// `=COMPLEXO(real; imaginário; [sufixo])` / `=COMPLEX(real_num; i_num; [suffix])` — monta o complexo em texto.
pub fn fn_complex(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 2, 3, |e: &Entradas<'_>| {
        let num_estrito = |i: usize| match e.valor(i) {
            Value::Bool(_) => Err(e_valor()),
            Value::Text(t) if t.is_empty() => Err(e_valor()),
            _ => e.num(i),
        };
        let (re, im) = (num_estrito(0)?, num_estrito(1)?);
        let sufixo = match e.texto_ou(2, "i")?.as_str() {
            "i" | "" => 'i',
            "j" => 'j',
            _ => return Err(e_valor()),
        };
        complexo_em_texto(Cx::new(re, im), sufixo)
    })
}

/// `=IMABS(número)` / `=IMABS(inumber)` — módulo do complexo.
pub fn fn_imabs(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    para_real(args, ev, |z| Ok(z.abs()))
}

/// `=IMAGINÁRIO(número)` / `=IMAGINARY(inumber)` — parte imaginária.
pub fn fn_imaginary(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    para_real(args, ev, |z| Ok(z.im))
}

/// `=IMREAL(número)` / `=IMREAL(inumber)` — parte real.
pub fn fn_imreal(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    para_real(args, ev, |z| Ok(z.re))
}

/// `=IMARG(número)` / `=IMARGUMENT(inumber)` — ângulo θ em radianos; zero dá #DIV/0!.
pub fn fn_imargument(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    para_real(args, ev, |z| if z.eh_zero() { Err(e_div0()) } else { Ok(z.arg()) })
}

/// `=IMCONJ(número)` / `=IMCONJUGATE(inumber)` — conjugado.
pub fn fn_imconjugate(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    unaria(args, ev, |z| Ok(Cx::new(z.re, -z.im)))
}

/// `=IMCOS(número)` / `=IMCOS(inumber)` — cosseno.
pub fn fn_imcos(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    unaria(args, ev, |z| Ok(z.cosseno()))
}

/// `=IMCOSH(número)` / `=IMCOSH(inumber)` — cosseno hiperbólico.
pub fn fn_imcosh(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    unaria(args, ev, |z| Ok(z.cosseno_h()))
}

/// `=IMCOT(número)` / `=IMCOT(inumber)` — cotangente.
pub fn fn_imcot(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    unaria(args, ev, |z| if z.eh_zero() { Err(e_num()) } else { z.cosseno().dividido(z.seno()) })
}

/// `=IMCOSEC(número)` / `=IMCSC(inumber)` — cossecante.
pub fn fn_imcsc(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    unaria(args, ev, |z| if z.eh_zero() { Err(e_num()) } else { z.seno().inverso() })
}

/// `=IMCOSECH(número)` / `=IMCSCH(inumber)` — cossecante hiperbólica.
pub fn fn_imcsch(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    unaria(args, ev, |z| if z.eh_zero() { Err(e_num()) } else { z.seno_h().inverso() })
}

/// `=IMSEC(número)` / `=IMSEC(inumber)` — secante.
pub fn fn_imsec(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    unaria(args, ev, |z| z.cosseno().inverso())
}

/// `=IMSECH(número)` / `=IMSECH(inumber)` — secante hiperbólica.
pub fn fn_imsech(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    unaria(args, ev, |z| z.cosseno_h().inverso())
}

/// `=IMSENO(número)` / `=IMSIN(inumber)` — seno.
pub fn fn_imsin(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    unaria(args, ev, |z| Ok(z.seno()))
}

/// `=IMSENH(número)` / `=IMSINH(inumber)` — seno hiperbólico.
pub fn fn_imsinh(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    unaria(args, ev, |z| Ok(z.seno_h()))
}

/// `=IMTAN(número)` / `=IMTAN(inumber)` — tangente.
pub fn fn_imtan(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    unaria(args, ev, |z| z.seno().dividido(z.cosseno()))
}

/// `=IMEXP(número)` / `=IMEXP(inumber)` — exponencial.
pub fn fn_imexp(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    unaria(args, ev, |z| Ok(z.exp()))
}

/// `=IMLN(número)` / `=IMLN(inumber)` — logaritmo natural; zero dá #NÚM!.
pub fn fn_imln(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    unaria(args, ev, Cx::ln)
}

/// `=IMLOG10(número)` / `=IMLOG10(inumber)` — logaritmo na base 10.
pub fn fn_imlog10(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    unaria(args, ev, |z| {
        let l = z.ln()?;
        Ok(Cx::new(l.re / std::f64::consts::LN_10, l.im / std::f64::consts::LN_10))
    })
}

/// `=IMLOG2(número)` / `=IMLOG2(inumber)` — logaritmo na base 2.
pub fn fn_imlog2(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    unaria(args, ev, |z| {
        let l = z.ln()?;
        Ok(Cx::new(l.re / std::f64::consts::LN_2, l.im / std::f64::consts::LN_2))
    })
}

/// `=IMRAIZ(número)` / `=IMSQRT(inumber)` — raiz quadrada.
pub fn fn_imsqrt(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    unaria(args, ev, |z| Ok(z.raiz()))
}

/// `=IMPOT(número; potência)` / `=IMPOWER(inumber; number)` — potência real de um complexo.
pub fn fn_impower(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 2, 2, |e| {
        let (z, s) = complexo_de(e.valor(0))?;
        if matches!(e.valor(1), Value::Bool(_)) {
            return Err(e_valor());
        }
        let n = e.num(1)?;
        complexo_em_texto(z.potencia(n)?, s.unwrap_or('i'))
    })
}

/// `=IMDIV(número1; número2)` / `=IMDIV(inumber1; inumber2)` — quociente; divisor zero dá #NÚM!.
pub fn fn_imdiv(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    binaria(args, ev, |a, b| a.dividido(b))
}

/// `=IMSUBTR(número1; número2)` / `=IMSUB(inumber1; inumber2)` — diferença.
pub fn fn_imsub(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    binaria(args, ev, |a, b| Ok(a.menos(b)))
}

/// `=IMSOMA(número1; [número2]; ...)` / `=IMSUM(inumber1; [inumber2]; ...)` — soma de vários complexos.
pub fn fn_imsum(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    acumula(args, ev, Cx::new(0.0, 0.0), Cx::soma)
}

/// `=IMPROD(número1; [número2]; ...)` / `=IMPRODUCT(inumber1; [inumber2]; ...)` — produto de vários complexos.
pub fn fn_improduct(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    acumula(args, ev, Cx::new(1.0, 0.0), Cx::vezes)
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::planilha::sheet::Sheet;

    fn calc(formula: &str) -> Value {
        let mut s = Sheet::new("Plan1");
        s.set_cell_raw(0, 0, formula.into(), None);
        s.get_value(0, 0)
    }

    fn txt(formula: &str, esperado: &str) {
        assert_eq!(calc(formula), Value::texto(esperado), "{formula}");
    }

    #[test]
    fn leitura_do_texto() {
        assert_eq!(complexo_do_texto("3+4i"), Ok((Cx::new(3.0, 4.0), Some('i'))));
        assert_eq!(complexo_do_texto("-2j"), Ok((Cx::new(0.0, -2.0), Some('j'))));
        assert_eq!(complexo_do_texto("i"), Ok((Cx::new(0.0, 1.0), Some('i'))));
        assert_eq!(complexo_do_texto("1,5-i"), Ok((Cx::new(1.5, -1.0), Some('i'))));
        assert_eq!(complexo_do_texto("1e2+3e-1i"), Ok((Cx::new(100.0, 0.3), Some('i'))));
        assert_eq!(complexo_do_texto("7"), Ok((Cx::new(7.0, 0.0), None)));
        assert!(complexo_do_texto("3+4k").is_err());
        assert!(complexo_do_texto("3 + 4i").is_err());
    }

    #[test]
    fn montar_e_partes() {
        txt("=COMPLEXO(3; 4)", "3+4i");
        txt("=COMPLEXO(3; 4; \"j\")", "3+4j");
        txt("=COMPLEXO(0; 1)", "i");
        txt("=COMPLEXO(1,5; -1)", "1,5-i");
        assert_eq!(calc("=IMABS(\"5+12i\")"), Value::Number(13.0));
        assert_eq!(calc("=IMAGINÁRIO(\"3+4i\")"), Value::Number(4.0));
        assert_eq!(calc("=IMREAL(\"6-9i\")"), Value::Number(6.0));
        txt("=IMCONJ(\"3+4i\")", "3-4i");
    }

    #[test]
    fn contas() {
        txt("=IMDIV(\"-238+240i\"; \"10+24i\")", "5+12i");
        txt("=IMPROD(\"3+4i\"; \"5-3i\")", "27+11i");
        txt("=IMSOMA(\"3+4i\"; \"5-3i\")", "8+i");
        txt("=IMSUBTR(\"13+4i\"; \"5+3i\")", "8+i");
        txt("=IMPOT(\"2+3i\"; 3)", "-46+9,00000000000001i");
        txt("=IMRAIZ(\"1+i\")", "1,09868411346781+0,455089860562227i");
        txt("=IMLN(\"3+4i\")", "1,6094379124341+0,927295218001612i");
        assert!(matches!(calc("=IMSOMA(\"1+i\"; \"1+j\")"), Value::Error(_)));
    }
}
