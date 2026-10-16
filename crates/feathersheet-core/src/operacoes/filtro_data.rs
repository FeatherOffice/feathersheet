//! Filtros de data por período do AutoFiltro ("Filtros de data" do Excel) e
//! "acima/abaixo da média" — o `<dynamicFilter>` do `.xlsx`.
//!
//! O "hoje" é sempre um argumento (número serial inteiro), nunca o relógio:
//! assim o resultado é testável e a GUI decide o fuso. Semanas começam no
//! domingo, como no Excel em pt-BR.

use crate::celulas::date_serial::serial_to_ymd;

/// Os tipos de `dynamicFilter` do Excel que o motor entende.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dinamico {
    Hoje,
    Ontem,
    Amanha,
    EstaSemana,
    SemanaPassada,
    ProximaSemana,
    EsteMes,
    MesPassado,
    ProximoMes,
    EsteTrimestre,
    TrimestrePassado,
    ProximoTrimestre,
    EsteAno,
    AnoPassado,
    ProximoAno,
    /// Do 1º de janeiro até hoje.
    AnoAteHoje,
    /// Todas as datas do trimestre 1 a 4, de qualquer ano.
    Trimestre(u8),
    /// Todas as datas do mês 1 a 12, de qualquer ano.
    Mes(u8),
    AcimaDaMedia,
    AbaixoDaMedia,
}

const NOMES: [(&str, Dinamico); 18] = [
    ("today", Dinamico::Hoje),
    ("yesterday", Dinamico::Ontem),
    ("tomorrow", Dinamico::Amanha),
    ("thisWeek", Dinamico::EstaSemana),
    ("lastWeek", Dinamico::SemanaPassada),
    ("nextWeek", Dinamico::ProximaSemana),
    ("thisMonth", Dinamico::EsteMes),
    ("lastMonth", Dinamico::MesPassado),
    ("nextMonth", Dinamico::ProximoMes),
    ("thisQuarter", Dinamico::EsteTrimestre),
    ("lastQuarter", Dinamico::TrimestrePassado),
    ("nextQuarter", Dinamico::ProximoTrimestre),
    ("thisYear", Dinamico::EsteAno),
    ("lastYear", Dinamico::AnoPassado),
    ("nextYear", Dinamico::ProximoAno),
    ("yearToDate", Dinamico::AnoAteHoje),
    ("aboveAverage", Dinamico::AcimaDaMedia),
    ("belowAverage", Dinamico::AbaixoDaMedia),
];

impl Dinamico {
    /// O `type` do `<dynamicFilter>` no `.xlsx` (`thisMonth`, `Q3`, `M12`…).
    pub fn nome_xlsx(&self) -> String {
        match self {
            Dinamico::Trimestre(q) => format!("Q{q}"),
            Dinamico::Mes(m) => format!("M{m}"),
            outro => NOMES.iter().find(|(_, d)| d == outro).map(|(n, _)| (*n).to_string()).unwrap_or_default(),
        }
    }

    /// O inverso de [`Self::nome_xlsx`]; `None` para tipo desconhecido.
    pub fn de_nome_xlsx(nome: &str) -> Option<Self> {
        if let Some(n) = NOMES.iter().find(|(n, _)| *n == nome) {
            return Some(n.1);
        }
        let (letra, resto) = nome.split_at(nome.len().min(1));
        let n: u8 = resto.parse().ok()?;
        match letra {
            "Q" if (1..=4).contains(&n) => Some(Dinamico::Trimestre(n)),
            "M" if (1..=12).contains(&n) => Some(Dinamico::Mes(n)),
            _ => None,
        }
    }

    /// Se depende da média da coluna (e não de datas).
    pub fn e_media(&self) -> bool {
        matches!(self, Dinamico::AcimaDaMedia | Dinamico::AbaixoDaMedia)
    }
}

/// Dias desde 1970-01-01 de uma data civil (algoritmo de Howard Hinnant).
fn dias_civis(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// O número serial do Excel (dias desde 30/12/1899) de uma data.
pub fn serial_de_data(ano: i32, mes: u32, dia: u32) -> i64 {
    dias_civis(ano as i64, mes as i64, dia as i64) - dias_civis(1899, 12, 30)
}

/// Dia da semana de um serial: 0 = domingo … 6 = sábado.
pub fn dia_da_semana(serial: i64) -> i64 {
    // 1970-01-01 foi quinta-feira.
    (serial + dias_civis(1899, 12, 30) + 4).rem_euclid(7)
}

/// Se o serial `valor` cai no período `tipo`, sendo `hoje` o serial de hoje.
/// As médias não passam por aqui (ver [`Dinamico::e_media`]).
pub fn no_periodo(tipo: Dinamico, valor: f64, hoje: i64) -> bool {
    if !valor.is_finite() {
        return false;
    }
    let v = valor.floor() as i64;
    let (vy, vm, _) = serial_to_ymd(v);
    let (hy, hm, _) = serial_to_ymd(hoje);
    let mes_de = |y: i32, m: u32| y as i64 * 12 + (m as i64 - 1);
    let tri_de = |y: i32, m: u32| y as i64 * 4 + (m as i64 - 1) / 3;
    let inicio_semana = hoje - dia_da_semana(hoje);
    let semana = |delta: i64| {
        let ini = inicio_semana + delta * 7;
        (ini..ini + 7).contains(&v)
    };
    match tipo {
        Dinamico::Hoje => v == hoje,
        Dinamico::Ontem => v == hoje - 1,
        Dinamico::Amanha => v == hoje + 1,
        Dinamico::EstaSemana => semana(0),
        Dinamico::SemanaPassada => semana(-1),
        Dinamico::ProximaSemana => semana(1),
        Dinamico::EsteMes => mes_de(vy, vm) == mes_de(hy, hm),
        Dinamico::MesPassado => mes_de(vy, vm) == mes_de(hy, hm) - 1,
        Dinamico::ProximoMes => mes_de(vy, vm) == mes_de(hy, hm) + 1,
        Dinamico::EsteTrimestre => tri_de(vy, vm) == tri_de(hy, hm),
        Dinamico::TrimestrePassado => tri_de(vy, vm) == tri_de(hy, hm) - 1,
        Dinamico::ProximoTrimestre => tri_de(vy, vm) == tri_de(hy, hm) + 1,
        Dinamico::EsteAno => vy == hy,
        Dinamico::AnoPassado => vy == hy - 1,
        Dinamico::ProximoAno => vy == hy + 1,
        Dinamico::AnoAteHoje => vy == hy && v <= hoje,
        Dinamico::Trimestre(q) => (vm - 1) / 3 + 1 == q as u32,
        Dinamico::Mes(m) => vm == m as u32,
        Dinamico::AcimaDaMedia | Dinamico::AbaixoDaMedia => false,
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    // Hoje = quinta-feira, 15/10/2026.
    fn hoje() -> i64 {
        serial_de_data(2026, 10, 15)
    }
    fn d(y: i32, m: u32, dia: u32) -> f64 {
        serial_de_data(y, m, dia) as f64
    }

    #[test]
    fn serial_e_dia_da_semana_batem_com_o_excel() {
        assert_eq!(serial_de_data(2000, 1, 1), 36526);
        assert_eq!(serial_to_ymd(hoje()), (2026, 10, 15));
        assert_eq!(dia_da_semana(hoje()), 4, "quinta");
        assert_eq!(dia_da_semana(serial_de_data(2026, 10, 18)), 0, "domingo");
    }

    #[test]
    fn dia_e_semana() {
        assert!(no_periodo(Dinamico::Hoje, d(2026, 10, 15) + 0.75, hoje()), "hora do dia não importa");
        assert!(no_periodo(Dinamico::Ontem, d(2026, 10, 14), hoje()));
        assert!(no_periodo(Dinamico::Amanha, d(2026, 10, 16), hoje()));
        // Semana de domingo 11/10 a sábado 17/10.
        assert!(no_periodo(Dinamico::EstaSemana, d(2026, 10, 11), hoje()));
        assert!(no_periodo(Dinamico::EstaSemana, d(2026, 10, 17), hoje()));
        assert!(!no_periodo(Dinamico::EstaSemana, d(2026, 10, 18), hoje()));
        assert!(!no_periodo(Dinamico::EstaSemana, d(2026, 10, 10), hoje()));
        assert!(no_periodo(Dinamico::SemanaPassada, d(2026, 10, 10), hoje()));
        assert!(no_periodo(Dinamico::SemanaPassada, d(2026, 10, 4), hoje()));
        assert!(no_periodo(Dinamico::ProximaSemana, d(2026, 10, 18), hoje()));
        assert!(no_periodo(Dinamico::ProximaSemana, d(2026, 10, 24), hoje()));
        assert!(!no_periodo(Dinamico::ProximaSemana, d(2026, 10, 25), hoje()));
    }

    #[test]
    fn mes_trimestre_e_ano() {
        assert!(no_periodo(Dinamico::EsteMes, d(2026, 10, 1), hoje()));
        assert!(!no_periodo(Dinamico::EsteMes, d(2025, 10, 15), hoje()), "outubro de outro ano");
        assert!(no_periodo(Dinamico::MesPassado, d(2026, 9, 30), hoje()));
        assert!(no_periodo(Dinamico::ProximoMes, d(2026, 11, 1), hoje()));
        assert!(no_periodo(Dinamico::EsteTrimestre, d(2026, 10, 1), hoje()), "T4: out a dez");
        assert!(!no_periodo(Dinamico::EsteTrimestre, d(2026, 7, 1), hoje()), "T3 já passou");
        assert!(no_periodo(Dinamico::EsteTrimestre, d(2026, 12, 31), hoje()));
        assert!(!no_periodo(Dinamico::EsteTrimestre, d(2026, 9, 30), hoje()));
        assert!(no_periodo(Dinamico::TrimestrePassado, d(2026, 9, 30), hoje()));
        assert!(no_periodo(Dinamico::ProximoTrimestre, d(2027, 1, 1), hoje()));
        assert!(no_periodo(Dinamico::EsteAno, d(2026, 1, 1), hoje()));
        assert!(no_periodo(Dinamico::AnoPassado, d(2025, 12, 31), hoje()));
        assert!(no_periodo(Dinamico::ProximoAno, d(2027, 6, 1), hoje()));
    }

    #[test]
    fn virada_de_ano_nos_relativos() {
        let janeiro = serial_de_data(2027, 1, 5);
        assert!(no_periodo(Dinamico::MesPassado, d(2026, 12, 20), janeiro));
        assert!(no_periodo(Dinamico::TrimestrePassado, d(2026, 11, 3), janeiro));
        assert!(no_periodo(Dinamico::SemanaPassada, d(2026, 12, 31), janeiro), "5/1/2027 é terça; semana passada 27/12 a 2/1");
    }

    #[test]
    fn acumulado_no_ano_e_todos_de_mes_ou_trimestre() {
        assert!(no_periodo(Dinamico::AnoAteHoje, d(2026, 1, 1), hoje()));
        assert!(no_periodo(Dinamico::AnoAteHoje, d(2026, 10, 15), hoje()));
        assert!(!no_periodo(Dinamico::AnoAteHoje, d(2026, 10, 16), hoje()));
        assert!(!no_periodo(Dinamico::AnoAteHoje, d(2025, 12, 31), hoje()));
        assert!(no_periodo(Dinamico::Mes(2), d(1999, 2, 28), hoje()), "qualquer ano");
        assert!(!no_periodo(Dinamico::Mes(2), d(2026, 3, 1), hoje()));
        assert!(no_periodo(Dinamico::Trimestre(1), d(2031, 3, 31), hoje()));
        assert!(!no_periodo(Dinamico::Trimestre(1), d(2031, 4, 1), hoje()));
    }

    #[test]
    fn nomes_do_xlsx_ida_e_volta() {
        for (nome, d) in NOMES {
            assert_eq!(Dinamico::de_nome_xlsx(nome), Some(d));
            assert_eq!(d.nome_xlsx(), nome);
        }
        assert_eq!(Dinamico::de_nome_xlsx("Q3"), Some(Dinamico::Trimestre(3)));
        assert_eq!(Dinamico::Mes(12).nome_xlsx(), "M12");
        assert_eq!(Dinamico::de_nome_xlsx("M13"), None);
        assert_eq!(Dinamico::de_nome_xlsx("Q5"), None);
        assert_eq!(Dinamico::de_nome_xlsx("lixo"), None);
        assert_eq!(Dinamico::de_nome_xlsx(""), None);
    }
}
