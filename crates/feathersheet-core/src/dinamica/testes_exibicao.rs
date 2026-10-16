//! Confere, com uma tabela pequena, os números de cada opção de exibição da
//! dinâmica contra o que o Excel mostra.
//!
//! Tabela: Região, Produto, Vendas, Data
//!   Sul/Maçã 100, Sul/Banana 200, Norte/Maçã 150, Norte/Banana 300, Sul/Maçã 120
//! Linhas = Região, Colunas = Produto, Soma de Vendas:
//!
//! |        | Banana | Maçã | Total |
//! | Norte  |  300   | 150  |  450  |
//! | Sul    |  200   | 220  |  420  |
//! | Total  |  500   | 370  |  870  |

use super::dinamica_layout::{montar_com_extras, Opcoes, Resultado};
use super::exibicao::*;
use super::pivot::{AgregacaoOp, DinamicaConfig, MedidaDinamica};
use crate::celulas::refs::Rect;
use crate::celulas::values::{to_text, Value};

fn t(s: &str) -> Value {
    Value::Text(s.into())
}

fn dados() -> Vec<Vec<Value>> {
    let l = |r: &str, p: &str, v: f64, d: &str| vec![t(r), t(p), Value::Number(v), t(d)];
    vec![
        vec![t("Região"), t("Produto"), t("Vendas"), t("Data")],
        l("Sul", "Maçã", 100.0, "2026-01-15"),
        l("Sul", "Banana", 200.0, "2026-01-20"),
        l("Norte", "Maçã", 150.0, "2026-02-10"),
        l("Norte", "Banana", 300.0, "2026-05-12"),
        l("Sul", "Maçã", 120.0, "2027-03-01"),
    ]
}

fn config(linhas: Vec<usize>, colunas: Vec<usize>) -> DinamicaConfig {
    DinamicaConfig {
        origem: Rect::new(0, 0, 5, 3),
        linhas,
        colunas,
        valores: vec![MedidaDinamica { coluna: 2, op: AgregacaoOp::Soma }],
        agrupar_data: None,
    }
}

fn rodar(c: &DinamicaConfig, e: &Extras) -> Resultado {
    let d = dados();
    montar_com_extras(d.len(), |r, k| d[r].get(k).cloned().unwrap_or(Value::Empty), c, &Opcoes::antigas(), e)
}

/// O miolo (sem cabeçalho, sem rótulos) como texto com 4 casas.
fn miolo(r: &Resultado) -> Vec<Vec<String>> {
    let rot = r.colunas_de_rotulo;
    r.matriz.rows().iter().skip(1).map(|l| {
        l[rot..].iter().map(|v| match v {
            Value::Number(n) => format!("{:.4}", n),
            outro => to_text(outro),
        }).collect()
    }).collect()
}

fn com(m: MostrarComo) -> Extras {
    Extras { valores: vec![m], ..Extras::default() }
}

fn regiao(item: &str) -> ItemBase {
    ItemBase::Item { nome: item.into() }
}

const REGIAO: CampoBase = CampoBase { eixo: Eixo::Linhas, posicao: 0 };
const PRODUTO: CampoBase = CampoBase { eixo: Eixo::Colunas, posicao: 0 };

fn cruzada() -> DinamicaConfig {
    config(vec![0], vec![1])
}

fn assert_miolo(r: &Resultado, esperado: &[&[&str]]) {
    let m = miolo(r);
    let e: Vec<Vec<String>> = esperado.iter().map(|l| l.iter().map(|s| s.to_string()).collect()).collect();
    assert_eq!(m, e);
}

#[test]
fn padrao_continua_igual() {
    let r = rodar(&cruzada(), &Extras::default());
    assert_miolo(&r, &[
        &["300.0000", "150.0000", "450.0000"],
        &["200.0000", "220.0000", "420.0000"],
        &["500.0000", "370.0000", "870.0000"],
    ]);
    // E o mesmo que o motor antigo dá.
    let d = dados();
    let antigo = super::pivot::montar_dinamica(&d, &cruzada());
    assert_eq!(r.matriz.rows(), antigo.rows());
}

#[test]
fn perc_total_geral() {
    let r = rodar(&cruzada(), &com(MostrarComo::PercTotalGeral));
    assert_miolo(&r, &[
        &["0.3448", "0.1724", "0.5172"],
        &["0.2299", "0.2529", "0.4828"],
        &["0.5747", "0.4253", "1.0000"],
    ]);
}

#[test]
fn perc_total_da_coluna_e_da_linha() {
    let r = rodar(&cruzada(), &com(MostrarComo::PercTotalColuna));
    assert_miolo(&r, &[
        &["0.6000", "0.4054", "0.5172"],
        &["0.4000", "0.5946", "0.4828"],
        &["1.0000", "1.0000", "1.0000"],
    ]);
    let r = rodar(&cruzada(), &com(MostrarComo::PercTotalLinha));
    assert_miolo(&r, &[
        &["0.6667", "0.3333", "1.0000"],
        &["0.4762", "0.5238", "1.0000"],
        &["0.5747", "0.4253", "1.0000"],
    ]);
}

#[test]
fn perc_de_diferenca_e_perc_diferenca_de_um_item() {
    let r = rodar(&cruzada(), &com(MostrarComo::PercDe { campo: REGIAO, item: regiao("Norte") }));
    assert_miolo(&r, &[
        &["1.0000", "1.0000", "1.0000"],
        &["0.6667", "1.4667", "0.9333"],
        &["", "", ""],
    ]);
    let r = rodar(&cruzada(), &com(MostrarComo::DiferencaDe { campo: REGIAO, item: regiao("Norte") }));
    assert_miolo(&r, &[
        &["", "", ""],
        &["-100.0000", "70.0000", "-30.0000"],
        &["", "", ""],
    ]);
    let r = rodar(&cruzada(), &com(MostrarComo::PercDiferencaDe { campo: REGIAO, item: regiao("Norte") }));
    assert_miolo(&r, &[
        &["", "", ""],
        &["-0.3333", "0.4667", "-0.0667"],
        &["", "", ""],
    ]);
}

#[test]
fn anterior_e_seguinte() {
    // Diferença do produto anterior (Banana -> Maçã), linha a linha.
    let r = rodar(&cruzada(), &com(MostrarComo::DiferencaDe { campo: PRODUTO, item: ItemBase::Anterior }));
    assert_miolo(&r, &[
        &["", "-150.0000", ""],
        &["", "20.0000", ""],
        &["", "-130.0000", ""],
    ]);
    // Seguinte: a última coluna de produto fica em branco.
    let r = rodar(&cruzada(), &com(MostrarComo::PercDe { campo: PRODUTO, item: ItemBase::Seguinte }));
    assert_miolo(&r, &[
        &["2.0000", "", ""],
        &["0.9091", "", ""],
        &["1.3514", "", ""],
    ]);
}

#[test]
fn total_acumulado_e_perc_acumulado() {
    let r = rodar(&cruzada(), &com(MostrarComo::TotalAcumuladoEm { campo: REGIAO }));
    assert_miolo(&r, &[
        &["300.0000", "150.0000", "450.0000"],
        &["500.0000", "370.0000", "870.0000"],
        &["", "", ""],
    ]);
    let r = rodar(&cruzada(), &com(MostrarComo::PercTotalAcumuladoEm { campo: REGIAO }));
    assert_miolo(&r, &[
        &["0.6000", "0.4054", "0.5172"],
        &["1.0000", "1.0000", "1.0000"],
        &["", "", ""],
    ]);
    // Acumulado pelas colunas (Banana, Maçã): vira acumulado horizontal.
    let r = rodar(&cruzada(), &com(MostrarComo::TotalAcumuladoEm { campo: PRODUTO }));
    assert_miolo(&r, &[
        &["300.0000", "450.0000", ""],
        &["200.0000", "420.0000", ""],
        &["500.0000", "870.0000", ""],
    ]);
}

#[test]
fn classificar() {
    // Banana: Sul 200 < Norte 300; Maçã: Norte 150 < Sul 220; total: Sul 420 < Norte 450.
    let r = rodar(&cruzada(), &com(MostrarComo::ClassificarMenorMaior { campo: REGIAO }));
    assert_miolo(&r, &[
        &["2.0000", "1.0000", "2.0000"],
        &["1.0000", "2.0000", "1.0000"],
        &["", "", ""],
    ]);
    let r = rodar(&cruzada(), &com(MostrarComo::ClassificarMaiorMenor { campo: PRODUTO }));
    // Norte: Banana 300 (1), Maçã 150 (2); Sul: Maçã 220 (1), Banana 200 (2).
    assert_miolo(&r, &[
        &["1.0000", "2.0000", ""],
        &["2.0000", "1.0000", ""],
        &["1.0000", "2.0000", ""],
    ]);
}

#[test]
fn indice() {
    // Norte/Banana = 300 * 870 / (450 * 500) = 1,16; Norte/Maçã = 150 * 870 / (450 * 370) = 0,7838
    let r = rodar(&cruzada(), &com(MostrarComo::Indice));
    assert_miolo(&r, &[
        &["1.1600", "0.7838", "1.0000"],
        &["0.8286", "1.2317", "1.0000"],
        &["1.0000", "1.0000", "1.0000"],
    ]);
}

#[test]
fn formatos_sugeridos() {
    assert_eq!(MostrarComo::PercTotalGeral.formato_sugerido(), Some("0,00%"));
    assert_eq!(MostrarComo::ClassificarMenorMaior { campo: REGIAO }.formato_sugerido(), Some("0"));
    assert_eq!(MostrarComo::Normal.formato_sugerido(), None);
}

#[test]
fn totais_gerais_ligados_e_desligados_separadamente() {
    let c = cruzada();
    let sem_linha_do_fim = Extras { totais_gerais_colunas: false, ..Extras::default() };
    let r = rodar(&c, &sem_linha_do_fim);
    assert_eq!(r.matriz.height(), 3, "cabeçalho + Norte + Sul");
    assert_eq!(r.matriz.width(), 4, "a coluna Total Geral à direita continua");
    let sem_coluna_da_direita = Extras { totais_gerais_linhas: false, ..Extras::default() };
    let r = rodar(&c, &sem_coluna_da_direita);
    assert_eq!(r.matriz.height(), 4);
    assert_eq!(r.matriz.width(), 3);
    assert_eq!(to_text(r.matriz.at(3, 0)), "Total Geral");
    // Os dois desligados, e a porcentagem do total continua certa.
    let nenhum = Extras {
        totais_gerais_linhas: false,
        totais_gerais_colunas: false,
        valores: vec![MostrarComo::PercTotalGeral],
        ..Extras::default()
    };
    let r = rodar(&c, &nenhum);
    assert_eq!((r.matriz.height(), r.matriz.width()), (3, 3));
    assert_eq!(format!("{:.4}", miolo(&r)[0][0].parse::<f64>().unwrap()), "0.3448");
}

#[test]
fn classificar_itens_az_za_e_por_valor() {
    let c = config(vec![0], vec![]);
    let nomes = |e: &Extras| -> Vec<String> {
        let r = rodar(&c, e);
        let rows = r.matriz.rows();
        rows[1..rows.len() - 1].iter().map(|l| to_text(&l[0])).collect()
    };
    let ord = |o| Extras { linhas: vec![CampoExtra { agrupamento: None, ordenacao: o }], ..Extras::default() };
    assert_eq!(nomes(&ord(Ordenacao::Crescente)), ["Norte", "Sul"]);
    assert_eq!(nomes(&ord(Ordenacao::Decrescente)), ["Sul", "Norte"]);
    // Norte 450, Sul 420.
    assert_eq!(nomes(&ord(Ordenacao::PorValor { medida: 0, crescente: false })), ["Norte", "Sul"]);
    assert_eq!(nomes(&ord(Ordenacao::PorValor { medida: 0, crescente: true })), ["Sul", "Norte"]);
    // Colunas por valor: Banana 500, Maçã 370 -> do maior para o menor mantém, do menor inverte.
    let cc = cruzada();
    let e = Extras {
        colunas: vec![CampoExtra { agrupamento: None, ordenacao: Ordenacao::PorValor { medida: 0, crescente: true } }],
        ..Extras::default()
    };
    let r = rodar(&cc, &e);
    assert_eq!(to_text(r.matriz.at(0, 1)), "Maçã (Vendas)");
    assert_eq!(to_text(r.matriz.at(0, 2)), "Banana (Vendas)");
}

#[test]
fn agrupar_numero_em_faixas_na_ordem_numerica() {
    // Linhas = Vendas em faixas de 100 (0 a 400), contando linhas.
    let mut c = config(vec![2], vec![]);
    c.valores = vec![MedidaDinamica { coluna: 2, op: AgregacaoOp::Contagem }];
    let e = Extras {
        linhas: vec![CampoExtra {
            agrupamento: Some(Agrupamento::Faixas(Faixas { inicio: 0.0, fim: 400.0, intervalo: 100.0 })),
            ordenacao: Ordenacao::Padrao,
        }],
        ..Extras::default()
    };
    let r = rodar(&c, &e);
    let l: Vec<(String, String)> = r.matriz.rows()[1..].iter().map(|x| (to_text(&x[0]), to_text(&x[1]))).collect();
    assert_eq!(
        l,
        [("100-199", "3"), ("200-299", "1"), ("300-400", "1"), ("Total Geral", "5")]
            .map(|(a, b)| (a.to_string(), b.to_string()))
    );
}

#[test]
fn faixas_ordenam_pelo_numero_e_nao_pelo_texto() {
    let c = config(vec![0], vec![]);
    let dados = [
        vec![t("V")],
        vec![Value::Number(105.0)],
        vec![Value::Number(25.0)],
        vec![Value::Number(5.0)],
    ];
    let e = Extras {
        linhas: vec![CampoExtra {
            agrupamento: Some(Agrupamento::Faixas(Faixas { inicio: 0.0, fim: 200.0, intervalo: 10.0 })),
            ordenacao: Ordenacao::Padrao,
        }],
        ..Extras::default()
    };
    let mut cfg = c;
    cfg.origem = Rect::new(0, 0, 3, 0);
    cfg.valores = vec![MedidaDinamica { coluna: 0, op: AgregacaoOp::Soma }];
    let r = montar_com_extras(dados.len(), |r, k| dados[r][k].clone(), &cfg, &Opcoes::antigas(), &e);
    let nomes: Vec<String> = r.matriz.rows()[1..4].iter().map(|l| to_text(&l[0])).collect();
    assert_eq!(nomes, ["0-9", "20-29", "100-109"]);
}

#[test]
fn agrupar_data_por_ano_trimestre_mes_e_dia() {
    // Datas: 2026-01-15 (100), 2026-01-20 (200), 2026-02-10 (150), 2026-05-12 (300), 2027-03-01 (120)
    let mut c = config(vec![3], vec![]);
    let por = |p| Extras {
        linhas: vec![CampoExtra { agrupamento: Some(Agrupamento::Data { parte: p }), ordenacao: Ordenacao::Padrao }],
        ..Extras::default()
    };
    let lista = |r: &Resultado| -> Vec<(String, String)> {
        let rows = r.matriz.rows();
        rows[1..rows.len() - 1].iter().map(|x| (to_text(&x[0]), to_text(&x[1]))).collect()
    };
    let par = |v: &[(&str, &str)]| v.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect::<Vec<_>>();
    assert_eq!(lista(&rodar(&c, &por(ParteData::Ano))), par(&[("2026", "750"), ("2027", "120")]));
    assert_eq!(lista(&rodar(&c, &por(ParteData::Trimestre))), par(&[("T1", "570"), ("T2", "300")]));
    assert_eq!(
        lista(&rodar(&c, &por(ParteData::Mes))),
        par(&[("jan", "300"), ("fev", "150"), ("mar", "120"), ("mai", "300")]),
        "meses na ordem do calendário, não alfabética"
    );
    assert_eq!(lista(&rodar(&c, &por(ParteData::Dia))), par(&[("1", "120"), ("10", "150"), ("12", "300"), ("15", "100"), ("20", "200")]));
    // Ano > Trimestre: a mesma coluna duas vezes, cada uma com a sua parte.
    c.linhas = vec![3, 3];
    let e = Extras {
        linhas: vec![
            CampoExtra { agrupamento: Some(Agrupamento::Data { parte: ParteData::Ano }), ordenacao: Ordenacao::Padrao },
            CampoExtra { agrupamento: Some(Agrupamento::Data { parte: ParteData::Trimestre }), ordenacao: Ordenacao::Padrao },
        ],
        ..Extras::default()
    };
    let r = rodar(&c, &e);
    let l: Vec<Vec<String>> = r.matriz.rows()[1..r.matriz.height() - 1]
        .iter().map(|x| x.iter().map(to_text).collect()).collect();
    assert_eq!(l, [["2026", "T1", "450"], ["2026", "T2", "300"], ["2027", "T1", "120"]].map(|x| x.map(String::from).to_vec()));
}

#[test]
fn recolher_nao_quebra_com_extras_em_dois_niveis() {
    // Região > Produto, % do total da coluna sem campo em Colunas: cada linha / total.
    let c = config(vec![0, 1], vec![]);
    let r = rodar(&c, &com(MostrarComo::PercTotalGeral));
    let ultimo = r.matriz.rows().last().unwrap();
    assert_eq!(to_text(&ultimo[0]), "Total Geral");
    assert_eq!(to_text(&ultimo[2]), "1");
}
