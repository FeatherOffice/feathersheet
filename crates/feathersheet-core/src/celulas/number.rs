//! Número escrito e lido como o JavaScript faz.
//!
//! O motor era TypeScript, e quem decidia como um número aparece na célula era
//! o `String(n)` do JavaScript; quem decidia se o texto digitado vira número
//! era o `Number(t)`. As duas regras estão em planilha que já existe no disco
//! de quem usa, então elas vêm junto na mudança — usar o `{}` do Rust mudaria
//! resultado em silêncio:
//!
//! - `1e21` viraria `1000000000000000000000` (o JavaScript escreve `1e+21`);
//! - `0,0000001` viraria `0.0000001` (o JavaScript escreve `1e-7`).
//!
//! A regra implementada é a do ECMAScript, `Number::toString`.

/// Arredonda para `digits` algarismos significativos, como o `toPrecision` do
/// JavaScript seguido de `Number(...)`.
pub fn round_to_precision(n: f64, digits: u32) -> f64 {
    if !n.is_finite() || n == 0.0 {
        return n;
    }
    let casas = digits.saturating_sub(1) as usize;
    format!("{n:.casas$e}").parse().unwrap_or(n)
}

/// O mesmo texto que `String(n)` produziria no JavaScript.
pub fn to_js_string(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_string();
    }
    // Cobre o `-0`: no JavaScript, `String(-0)` é "0".
    if n == 0.0 {
        return "0".to_string();
    }
    if n.is_infinite() {
        return if n < 0.0 { "-Infinity" } else { "Infinity" }.to_string();
    }
    let corpo = ecma_positivo(n.abs());
    if n < 0.0 {
        format!("-{corpo}")
    } else {
        corpo
    }
}

/// O miolo da regra, para um número finito e maior que zero.
///
/// Entre 1e-6 e 1e21 o JavaScript escreve em notação comum, com os
/// algarismos mais curtos que voltam ao mesmo `f64` — exatamente o que o `{}`
/// do Rust escreve, sem passar pela científica. É o caso de quase todo
/// número de planilha, e o atalho corta ~180 ns por número (o exportador de
/// `.xlsx` escreve 59 milhões no `spotify_full`). O teste
/// `atalho_da_notacao_comum_bate_com_a_regra` confere contra o caminho longo.
fn ecma_positivo(a: f64) -> String {
    if (1e-6..1e21).contains(&a) {
        return format!("{a}");
    }
    ecma_positivo_pela_regra(a)
}

/// O caminho longo, que vale para qualquer número finito e positivo.
///
/// O `{:e}` do Rust já entrega a representação **mais curta** que volta ao
/// mesmo `f64` — a mesma exigência do ECMAScript ("k o menor possível"). Então
/// o trabalho aqui é só escolher entre notação comum e científica, com os
/// mesmos limites do JavaScript: científica abaixo de 1e-6 e a partir de 1e21.
fn ecma_positivo_pela_regra(a: f64) -> String {
    let cientifica = format!("{a:e}");
    let (mantissa, expoente) = cientifica.split_once('e').unwrap_or((&cientifica, "0"));
    let expoente: i32 = expoente.parse().unwrap_or(0);
    let digitos: String = mantissa.chars().filter(|c| *c != '.').collect();

    // `k` é quantos algarismos significativos há; `n` é onde a vírgula cai
    // (o valor é `0,digitos` vezes 10 elevado a `n`).
    let k = digitos.len() as i32;
    let n = expoente + 1;

    if k <= n && n <= 21 {
        // 100 -> "1" + "00"
        let mut s = digitos;
        for _ in 0..(n - k) {
            s.push('0');
        }
        s
    } else if 0 < n && n <= 21 {
        // 1,5 -> "1" "." "5"
        let corte = n as usize;
        format!("{}.{}", &digitos[..corte], &digitos[corte..])
    } else if -6 < n && n <= 0 {
        // 0,0001 -> "0." + "000" + "1"
        let mut s = String::from("0.");
        for _ in 0..(-n) {
            s.push('0');
        }
        s.push_str(&digitos);
        s
    } else {
        // Científica: um algarismo, vírgula, o resto, e o expoente com sinal.
        let e = n - 1;
        let sinal = if e < 0 { '-' } else { '+' };
        let e = e.abs();
        if k == 1 {
            format!("{digitos}e{sinal}{e}")
        } else {
            format!("{}.{}e{}{}", &digitos[..1], &digitos[1..], sinal, e)
        }
    }
}

/// Como o número aparece na célula: sem notação científica à toa e sem o lixo
/// de ponto flutuante (`0,1 + 0,2` mostra `0,3`).
///
/// Tradução de `formatNumber` em `core/values.ts`. Os doze algarismos são o
/// que esconde o erro de arredondamento sem inventar precisão.
pub fn format_number(n: f64) -> String {
    if !n.is_finite() {
        return "#NUM!".to_string();
    }
    if n.fract() == 0.0 && n.abs() < 1e15 {
        return to_js_string(n);
    }
    to_js_string(round_to_precision(n, 12))
}

/// O mesmo que `Number(texto)` no JavaScript, mas só para o que interessa a
/// uma planilha: devolve `None` no lugar de `NaN`, e também no lugar de
/// infinito — quem chama sempre exigia `Number.isFinite` depois.
///
/// O `parse::<f64>()` do Rust aceita `"inf"`, `"infinity"` e `"nan"`, que o
/// `Number()` recusa; como o resultado deles não é finito, a exigência de ser
/// finito já dá conta dos três.
pub fn from_js_string(texto: &str) -> Option<f64> {
    let t = texto.trim();
    if t.is_empty() {
        return Some(0.0);
    }
    // O `Number()` entende as bases do JavaScript; o `parse` do Rust não.
    if let Some(n) = base_diferente(t) {
        return Some(n);
    }
    // O `Number()` não aceita separador de milhar nem sublinhado.
    if t.contains('_') {
        return None;
    }
    let n: f64 = t.parse().ok()?;
    if n.is_finite() {
        Some(n)
    } else {
        None
    }
}

/// `0x1A`, `0o17`, `0b1010` — como o `Number()` os lê.
fn base_diferente(t: &str) -> Option<f64> {
    let (resto, base) = if let Some(r) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        (r, 16)
    } else if let Some(r) = t.strip_prefix("0o").or_else(|| t.strip_prefix("0O")) {
        (r, 8)
    } else if let Some(r) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
        (r, 2)
    } else {
        return None;
    };
    if resto.is_empty() {
        return None;
    }
    u128::from_str_radix(resto, base).ok().map(|v| v as f64)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn atalho_da_notacao_comum_bate_com_a_regra() {
        let mut bordas = vec![
            1e-6, 1e21, 0.1 + 0.2, 1.5, 100.0, 123_456_789_012_345_680_000.0, 0.000_001_5, 9_007_199_254_740_993.0,
            f64::EPSILON, 5e-324, f64::MAX, 1.0 / 3.0, 46_288.0, 0.735, 2.5e-7,
        ];
        for b in bordas.clone() {
            bordas.push(f64::from_bits(b.to_bits() + 1));
            bordas.push(f64::from_bits(b.to_bits() - 1));
        }
        // Xorshift: bits aleatórios (todos os expoentes) e números "de
        // planilha" (inteiros, duas casas, frações pequenas).
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut proximo = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        for _ in 0..2_000_000 {
            let bits = proximo();
            bordas.push(f64::from_bits(bits >> 1)); // positivo, qualquer expoente
            let m = (proximo() % 10_000_000_000) as f64;
            bordas.push(m / 100.0);
            bordas.push(m / 1e9);
            bordas.push(m * 1e10);
        }
        for a in bordas {
            if a.is_finite() && a > 0.0 {
                assert_eq!(ecma_positivo(a), ecma_positivo_pela_regra(a), "{a:e}");
            }
        }
    }

    #[test]
    fn escreve_inteiro_como_o_javascript() {
        assert_eq!(to_js_string(0.0), "0");
        assert_eq!(to_js_string(-0.0), "0");
        assert_eq!(to_js_string(1.0), "1");
        assert_eq!(to_js_string(100.0), "100");
        assert_eq!(to_js_string(-42.0), "-42");
        assert_eq!(to_js_string(1e15), "1000000000000000");
    }

    #[test]
    fn escreve_fracao_como_o_javascript() {
        assert_eq!(to_js_string(1.5), "1.5");
        assert_eq!(to_js_string(0.1), "0.1");
        assert_eq!(to_js_string(-0.25), "-0.25");
        assert_eq!(to_js_string(0.0001), "0.0001");
        assert_eq!(to_js_string(0.000001), "0.000001");
    }

    #[test]
    fn troca_para_cientifica_nos_mesmos_limites_do_javascript() {
        // Abaixo de 1e-6 e a partir de 1e21 o JavaScript escreve com expoente.
        assert_eq!(to_js_string(1e-7), "1e-7");
        assert_eq!(to_js_string(1.5e-7), "1.5e-7");
        assert_eq!(to_js_string(1e20), "100000000000000000000");
        assert_eq!(to_js_string(1e21), "1e+21");
        assert_eq!(to_js_string(1.25e21), "1.25e+21");
        assert_eq!(to_js_string(-1e21), "-1e+21");
    }

    #[test]
    fn arredonda_por_algarismo_significativo() {
        assert_eq!(round_to_precision(1.0 / 3.0, 12), 0.333333333333);
        // O clássico: 0,1 + 0,2 volta a ser 0,3 depois de 12 algarismos.
        assert_eq!(to_js_string(round_to_precision(0.1 + 0.2, 12)), "0.3");
    }

    #[test]
    fn le_numero_como_o_javascript() {
        assert_eq!(from_js_string("12"), Some(12.0));
        assert_eq!(from_js_string("  -3.5  "), Some(-3.5));
        assert_eq!(from_js_string(""), Some(0.0));
        assert_eq!(from_js_string("1e3"), Some(1000.0));
        assert_eq!(from_js_string("0x1A"), Some(26.0));
        assert_eq!(from_js_string(".5"), Some(0.5));
        assert_eq!(from_js_string("5."), Some(5.0));
    }

    #[test]
    fn recusa_o_que_o_javascript_recusa() {
        assert_eq!(from_js_string("abc"), None);
        assert_eq!(from_js_string("1_0"), None);
        assert_eq!(from_js_string("12 34"), None);
        // O Rust aceitaria estes três; o `Number()` não.
        assert_eq!(from_js_string("inf"), None);
        assert_eq!(from_js_string("infinity"), None);
        assert_eq!(from_js_string("nan"), None);
        assert_eq!(from_js_string("Infinity"), None);
    }

    /// A representação mais curta é exigência do ECMAScript e é o que o `{:e}`
    /// do Rust promete. Se algum dia deixar de valer, o número na célula ganha
    /// zeros à toa — então fica travado por teste.
    #[test]
    fn ida_e_volta_nao_perde_nada() {
        for n in [
            1.0,
            0.1,
            1.0 / 3.0,
            1.7976931348623157e308,
            5e-324,
            123456.789,
            -98765.4321,
        ] {
            let texto = to_js_string(n);
            assert_eq!(texto.parse::<f64>().unwrap(), n, "voltou diferente: {texto}");
        }
    }
}
