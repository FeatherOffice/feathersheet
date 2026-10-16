//! CHT-017: confronta a extração do FeatherSheet com o contrato Charts.
//! Não reimplementa contas: compara com o extrator real da origem.
use std::collections::{BTreeMap, HashMap};
use feathersheet_core::{Value, grafico::chart::{
    extract_chart_data_with, AggregateFn, ChartConfig, ChartRange, ChartStyle,
    ChartType, DateHierarchy,
}};
use feathersheet_io::grafico_compartilhado::{
    extrair_grafico_com, preparar_grafico, ErroAdaptacaoGrafico,
};
use feather_charts::{ContratoGrafico, Tipo};
use feather_draw::Cor;

fn dados() -> HashMap<(u32,u32), Value> {
    let mut m = HashMap::new();
    for (c, h) in ["Grupo", "Vendas", "Meta"].iter().enumerate() {
        m.insert((0, c as u32), Value::Text((*h).into()));
    }
    for (i, (grupo, a, b)) in [
        ("A", 10.0, 1.0), ("B", 20.0, 2.0),
        ("A", 30.0, 3.0), ("B", 40.0, 4.0),
    ].iter().enumerate() {
        let r = i as u32 + 1;
        m.insert((r, 0), Value::Text((*grupo).into()));
        m.insert((r, 1), Value::Number(*a));
        m.insert((r, 2), Value::Number(*b));
    }
    m
}
fn config(tipo: ChartType) -> ChartConfig {
    ChartConfig {
        tipo, title: "Receita regional".into(),
        range: ChartRange { r0: 0, c0: 0, r1: 4, c1: 2 },
        series_in_columns: true, has_headers: true,
        has_category_labels: true, aggregate: true, ..Default::default()
    }
}
fn comparar(origem: &HashMap<(u32,u32), Value>, c: &ChartConfig) -> ContratoGrafico {
    let ler = |r, col| origem.get(&(r, col)).cloned().unwrap_or(Value::Empty);
    let legado = extract_chart_data_with(&ler, c);
    let comum = extrair_grafico_com(&ler, c).unwrap();
    assert_eq!(comum.grafico.titulo, legado.title);
    if c.tipo == ChartType::Scatter {
        let eixos = legado.x_values.as_ref().unwrap();
        assert_eq!(comum.grafico.categorias.len(), eixos.len());
        for (txt, x) in comum.grafico.categorias.iter().zip(eixos) {
            assert_eq!(txt.parse::<f64>().unwrap(), *x);
        }
    } else {
        assert_eq!(comum.grafico.categorias, legado.categories);
    }
    assert_eq!(comum.grafico.series.len(), legado.series.len());
    for (s, antigo) in comum.grafico.series.iter().zip(&legado.series) {
        assert_eq!(s.nome, antigo.name);
        assert_eq!(s.valores, antigo.values);
        assert_eq!(s.valores.iter().sum::<f64>(), antigo.values.iter().sum::<f64>());
    }
    assert_eq!(comum.avisos, legado.aviso.iter().cloned().collect::<Vec<_>>());
    assert_eq!(comum.amostra.map(|a| (a.disponiveis, a.usados)),
               legado.amostrado.map(|a| (a.disponiveis, a.usados)));
    comum.validar().unwrap();
    comum
}

#[test]
fn seis_tipos_preservam_titulos_dados_cores_escala_e_svg_pdf() {
    let origem = dados();
    let anterior = origem.clone();
    for (tipo, comum) in [
        (ChartType::Column, Tipo::Colunas),
        (ChartType::Bar, Tipo::Barras),
        (ChartType::Line, Tipo::Linhas),
        (ChartType::Area, Tipo::Area),
        (ChartType::StepLine, Tipo::Degraus),
        (ChartType::Pie, Tipo::Pizza),
    ] {
        let mut c = config(tipo);
        if tipo == ChartType::Pie { c.range.c1 = 1; }
        let saida = comparar(&origem, &c);
        assert_eq!(saida.grafico.tipo, comum);
        assert_eq!(saida.grafico.titulo, "Receita regional");
        assert_eq!(saida.metadados_series.len(), saida.grafico.series.len());
        for (s, metadado) in saida.grafico.series.iter().zip(&saida.metadados_series) {
            assert_eq!(s.cor, metadado.cor);
        }
        let svg = saida.grafico.exportar_svg(640.0, 400.0).unwrap();
        let pdf = saida.grafico.exportar_pdf(640.0, 400.0).unwrap();
        assert!(svg.contains("<path"));
        assert!(pdf.starts_with(b"%PDF-1.4"));
        if tipo == ChartType::Pie {
            assert!(saida.grafico.eixos(640.0, 400.0).unwrap().is_none());
        } else {
            let eixo = saida.grafico.eixos(640.0, 400.0).unwrap().unwrap();
            assert!(eixo.zero.is_finite());
            assert!(eixo.area.largura > 0.0);
        }
    }
    assert_eq!(origem, anterior);
}

#[test]
fn sete_agregacoes_delegadas_ao_feathersheet_sem_recontar() {
    let origem = dados();
    for funcao in [
        AggregateFn::Sum, AggregateFn::Avg, AggregateFn::Count,
        AggregateFn::Min, AggregateFn::Max, AggregateFn::Stddev,
        AggregateFn::Mode,
    ] {
        let mut c = config(ChartType::Column);
        c.aggregate_fn = Some(funcao);
        c.aggregate_by_series.insert(1, AggregateFn::Sum);
        let contrato = comparar(&origem, &c);
        assert_eq!(contrato.metadados_series[0].agregacao.as_deref(), Some(funcao.rotulo()));
        assert_eq!(contrato.metadados_series[1].agregacao.as_deref(), Some("Soma"));
        assert_eq!(contrato.metadados_series[0].indice_origem, 0);
        assert_eq!(contrato.metadados_series[1].indice_origem, 1);
    }
}

#[test]
fn ocultacao_respeita_indice_e_cor_de_serie_no_svg() {
    let origem = dados();
    let mut c = config(ChartType::Bar);
    c.style = ChartStyle {
        hidden_series: vec![0],
        series_colors: BTreeMap::from([(1, "#21bcd0".into())]),
        ..Default::default()
    };
    let contrato = comparar(&origem, &c);
    assert_eq!(contrato.grafico.series.len(), 1);
    assert_eq!(contrato.metadados_series[0].indice_origem, 1);
    assert_eq!(contrato.grafico.series[0].cor, Cor::rgb(0x21, 0xbc, 0xd0));
    assert!(contrato.grafico.exportar_svg(640.0, 400.0).unwrap().contains("#21bcd0"));
}

#[test]
fn hierarquia_mes_agrega_na_origem_sem_perder_total() {
    let mut origem = dados();
    for (r, s) in [
        (1, "03/01/2026"), (2, "20/01/2026"),
        (3, "01/02/2026"), (4, "28/02/2026"),
    ] { origem.insert((r, 0), Value::Text(s.into())); }
    let mut c = config(ChartType::Line);
    c.aggregate = false;
    c.style.date_hierarchy = Some(DateHierarchy::Month);
    let contrato = comparar(&origem, &c);
    assert_eq!(contrato.grafico.categorias, ["01/2026", "02/2026"]);
    assert_eq!(contrato.grafico.series[0].valores, [30., 70.]);
    assert_eq!(contrato.metadados_series[0].agregacao.as_deref(), Some("Soma"));
}

#[test]
fn dispersao_mantem_x_e_pares_do_extrator() {
    let origem = dados();
    let mut c = config(ChartType::Scatter);
    c.aggregate = false;
    c.has_category_labels = false;
    let contrato = comparar(&origem, &c);
    assert_eq!(contrato.grafico.tipo, Tipo::Dispersao);
    assert_eq!(contrato.grafico.categorias, ["10", "20", "30", "40"]);
    assert_eq!(contrato.grafico.series[0].valores, [1., 2., 3., 4.]);
    assert!(contrato.cena(640.0, 400.0).is_ok());
}

#[test]
fn tipo_nao_suportado_ou_infinito_retorna_erro() {
    let origem = dados();
    let mut c = config(ChartType::Combo);
    let ler = |r, col| origem.get(&(r, col)).cloned().unwrap_or(Value::Empty);
    let d = extract_chart_data_with(&ler, &c);
    assert_eq!(preparar_grafico(&c, d), Err(ErroAdaptacaoGrafico::TipoNaoSuportado(ChartType::Combo)));
    c.tipo = ChartType::Column;
    let mut d = extract_chart_data_with(&ler, &c);
    d.series[0].values[0] = f64::INFINITY;
    assert!(preparar_grafico(&c, d).is_err());
}
