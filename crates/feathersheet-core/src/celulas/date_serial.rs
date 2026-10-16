//! Conversão entre data e serial numérico do Excel/FeatherSheet.

pub const DAY_MS: f64 = 86_400_000.0;

/// Época do Excel: 30/12/1899 em timestamp UNIX (milissegundos UTC).
/// UTC(1899, 11, 30) = -2209161600000 ms (25569 dias antes de 01/01/1970).
pub const EPOCH_MS: f64 = -2_209_161_600_000.0;

/// Converte timestamp UNIX (em milissegundos) para número serial de data.
pub fn timestamp_to_serial(ms: f64) -> f64 {
    (ms - EPOCH_MS) / DAY_MS
}

/// Arredondamento JS (Math.round), cuja metade negativa vai para +infinito.
/// Em Rust `f64::round` afasta de zero: -1,5 virava -2, mas no TS vira -1.
/// Não somar 0,5 diretamente a `n`: arredondaria incorretamente números
/// grandes e valores próximos da borda de precisão da mantissa.
fn round_javascript(n: f64) -> f64 {
    if !n.is_finite() {
        return n;
    }
    let base = n.floor();
    if n - base >= 0.5 { base + 1.0 } else { base }
}

/// Converte número serial de data para timestamp UNIX (em milissegundos).
/// Espelha `new Date(EPOCH + Math.round(serial * DAY_MS))` do TypeScript.
pub fn serial_to_timestamp(serial: f64) -> f64 {
    EPOCH_MS + round_javascript(serial * DAY_MS)
}

/// Converte número serial do Excel (dias desde 1899-12-30) em (Ano, Mês, Dia).
///
/// ⚠ A constante é 2_415_019, não 2_415_018. Com 2_415_018 **toda data do
/// aplicativo saía um dia atrasada** — o serial 1 virava 30/12/1899 em vez de
/// 31/12/1899, e 46000 virava 08/12/2025 em vez de 09/12/2025. O defeito foi
/// achado em 22/09/2026 comparando esta função com o `serialToDate` do
/// `date-serial.ts` rodando, valor a valor; nenhum teste daqui o pegava
/// porque todos usavam o caminho por timestamp, que é outro.
///
/// A conta: a época é 30/12/1899, que é o dia juliano 2.415.018,5. O serial 1
/// é o dia seguinte, então a parte inteira do dia juliano dele é 2.415.019.
pub fn serial_to_ymd(serial: i64) -> (i32, u32, u32) {
    let l = serial + 68569 + 2_415_019;
    let n = (4 * l) / 146097;
    let l = l - (146097 * n + 3) / 4;
    let i = (4000 * (l + 1)) / 1461001;
    let l = l - (1461 * i) / 4 + 31;
    let j = (80 * l) / 2447;
    let d = l - (2447 * j) / 80;
    let l = j / 11;
    let m = j + 2 - 12 * l;
    let y = 100 * (n - 49) + i + l;
    (y as i32, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_javascript_reproduz_as_metades_do_typescript() {
        // Math.round(-1.5) = -1, enquanto f64::round(-1.5) = -2.
        for (valor, esperado) in [
            (-2.5, -2.0), (-1.5, -1.0), (-0.5, 0.0),
            (0.5, 1.0), (1.5, 2.0), (2.5, 3.0),
            (-2.499, -2.0), (-2.501, -3.0),
        ] {
            assert_eq!(round_javascript(valor), esperado, "{valor}");
        }
        assert!(round_javascript(f64::NAN).is_nan());
        assert_eq!(round_javascript(f64::INFINITY), f64::INFINITY);
    }

    #[test]
    fn serial_negativo_com_meio_milissegundo() {
        assert_eq!(serial_to_timestamp(-1.5 / DAY_MS), EPOCH_MS - 1.0);
        assert_eq!(serial_to_timestamp(1.5 / DAY_MS), EPOCH_MS + 2.0);
        assert_eq!(serial_to_timestamp(-0.5 / DAY_MS), EPOCH_MS);
    }

    #[test]
    fn datas_excel_e_epoch_estao_corretas() {
        assert_eq!(serial_to_timestamp(0.0), EPOCH_MS);
        assert_eq!(serial_to_timestamp(1.0), EPOCH_MS + DAY_MS);
        assert_eq!(serial_to_timestamp(2.0), EPOCH_MS + 2.0 * DAY_MS);
        assert_eq!(serial_to_ymd(0), (1899, 12, 30));
        assert_eq!(serial_to_ymd(1), (1899, 12, 31));
        assert_eq!(serial_to_ymd(2), (1900, 1, 1));
        assert_eq!(serial_to_ymd(25_569), (1970, 1, 1));
    }

    #[test]
    fn test_date_serial_conversion() {
        let ms = 0.0; // 01/01/1970 00:00:00 UTC
        let serial = timestamp_to_serial(ms);
        assert!((serial - 25569.0).abs() < 1e-6);
        let back_ms = serial_to_timestamp(serial);
        assert_eq!(ms, back_ms);
    }
}
