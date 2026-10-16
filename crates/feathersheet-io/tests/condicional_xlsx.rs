//! Formatação condicional vai e volta do `.xlsx`: gravar com o `xlsx_write`,
//! reler com o `xlsx_read` e encontrar a mesma regra.
use feathersheet_core::operacoes::rules::{
    CellCondition, CellOp, ColorScaleCondition, CondStyle, Condition, ConditionalRule, DataBarCondition, Faixa, ScalePoint,
    ScalePointType,
};
use feathersheet_core::planilha::workbook::Workbook;
use feathersheet_io::xlsx_read::ler_xlsx_workbook;
use feathersheet_io::xlsx_write::{gerar_xlsx, montar_partes};

fn regras(w: &Workbook, aba: usize) -> Vec<ConditionalRule> {
    let r = w.aba(aba).unwrap().sheet.r.clone().unwrap_or_default();
    if r.is_empty() { return Vec::new(); }
    serde_json::from_str(&r).expect("JSON das regras")
}

fn volta(originais: &[ConditionalRule]) -> Vec<ConditionalRule> {
    let mut w = Workbook::new();
    w.criar_aba("Planilha1");
    w.aba_mut(0).unwrap().sheet.r = Some(serde_json::to_string(originais).unwrap());
    let bytes = gerar_xlsx(&w).expect("gravar");
    let (lida, _) = ler_xlsx_workbook(bytes).expect("ler");
    regras(&lida, 0)
}

fn estilo() -> CondStyle {
    CondStyle { bg: Some("#ffc7ce".into()), color: Some("#9c0006".into()), bold: Some(true), italic: Some(false), underline: Some(true), strike: None }
}

fn celula(i: usize, faixa: Faixa, op: CellOp, values: &[&str]) -> ConditionalRule {
    ConditionalRule {
        id: format!("cf_{i}"),
        ranges: vec![faixa],
        condition: Condition::Cell(CellCondition { op, values: values.iter().map(|s| s.to_string()).collect(), style: estilo() }),
    }
}

fn ponto(t: ScalePointType, v: Option<f64>, cor: &str) -> ScalePoint {
    ScalePoint { point_type: t, value: v, color: cor.into() }
}

#[test]
fn cada_tipo_de_regra_vai_e_volta() {
    let f = Faixa { r0: 1, c0: 2, r1: 40, c1: 2 };
    let mut originais = Vec::new();
    let casos: Vec<(CellOp, Vec<&str>)> = vec![
        (CellOp::GreaterThan, vec!["10"]),
        (CellOp::GreaterOrEqual, vec!["1,5"]),
        (CellOp::LessThan, vec!["-3"]),
        (CellOp::LessOrEqual, vec!["0,5"]),
        (CellOp::Equal, vec!["abc"]),
        (CellOp::NotEqual, vec!["7"]),
        (CellOp::Between, vec!["1", "9,5"]),
        (CellOp::NotBetween, vec!["2", "4"]),
        (CellOp::IsEmpty, vec![]),
        (CellOp::IsNotEmpty, vec![]),
        (CellOp::TextContains, vec!["a \"b\" & c"]),
        (CellOp::TextNotContains, vec!["xy"]),
        (CellOp::TextStartsWith, vec!["ab"]),
        (CellOp::TextEndsWith, vec!["yz"]),
        (CellOp::Duplicate, vec![]),
        (CellOp::Unique, vec![]),
        (CellOp::AboveAverage, vec![]),
        (CellOp::BelowAverage, vec![]),
        (CellOp::Formula, vec!["=E(ÉNÚM($C2);$C2>10,5;PROCURAR(\"x\";D2)>0)"]),
    ];
    for (i, (op, v)) in casos.iter().enumerate() {
        originais.push(celula(i + 1, f, *op, v));
    }
    let n = originais.len();
    originais.push(ConditionalRule {
        id: format!("cf_{}", n + 1),
        ranges: vec![Faixa { r0: 0, c0: 5, r1: 9, c1: 5 }],
        condition: Condition::ColorScale(ColorScaleCondition {
            min: ponto(ScalePointType::Min, None, "#f8696b"),
            mid: Some(ponto(ScalePointType::Percentile, Some(50.0), "#ffeb84")),
            max: ponto(ScalePointType::Max, None, "#63be7b"),
        }),
    });
    originais.push(ConditionalRule {
        id: format!("cf_{}", n + 2),
        ranges: vec![Faixa { r0: 0, c0: 6, r1: 9, c1: 6 }],
        condition: Condition::ColorScale(ColorScaleCondition {
            min: ponto(ScalePointType::Number, Some(0.0), "#ffffff"),
            mid: None,
            max: ponto(ScalePointType::Percent, Some(90.0), "#00aa00"),
        }),
    });
    originais.push(ConditionalRule {
        id: format!("cf_{}", n + 3),
        ranges: vec![Faixa { r0: 0, c0: 7, r1: 9, c1: 7 }],
        condition: Condition::DataBar(DataBarCondition {
            color: "#638ec6".into(),
            negative_color: None,
            hide_value: true,
            min: Some(ponto(ScalePointType::Number, Some(0.0), "#638ec6")),
            max: None,
        }),
    });
    let lidas = volta(&originais);
    assert_eq!(lidas.len(), originais.len());
    for (a, b) in originais.iter().zip(&lidas) {
        assert_eq!(a, b);
    }
}

#[test]
fn formula_em_varios_retangulos_vira_uma_regra_por_retangulo() {
    let regra = ConditionalRule {
        id: "cf_1".into(),
        ranges: vec![Faixa { r0: 0, c0: 0, r1: 2, c1: 0 }, Faixa { r0: 4, c0: 2, r1: 6, c1: 2 }],
        condition: Condition::Cell(CellCondition { op: CellOp::Formula, values: vec!["=A1>0".into()], style: estilo() }),
    };
    let lidas = volta(&[regra]);
    assert_eq!(lidas.len(), 2);
    for (l, (r0, c0)) in lidas.iter().zip([(0, 0), (4, 2)]) {
        assert_eq!(l.ranges.len(), 1);
        assert_eq!((l.ranges[0].r0, l.ranges[0].c0), (r0, c0));
        let Condition::Cell(c) = &l.condition else { panic!() };
        // No motor a fórmula é relativa ao canto de cada retângulo; no arquivo
        // cada retângulo é um `sqref` próprio, então a fórmula não muda.
        assert_eq!(c.values, vec!["=A1>0".to_string()]);
    }
}

#[test]
fn xml_tem_dxfs_e_fica_antes_da_validacao() {
    let regra = celula(1, Faixa { r0: 0, c0: 0, r1: 4, c1: 0 }, CellOp::GreaterThan, &["3"]);
    let mut w = Workbook::new();
    w.criar_aba("P");
    let aba = &mut w.aba_mut(0).unwrap().sheet;
    aba.r = Some(serde_json::to_string(&[regra]).unwrap());
    aba.va = Some(r#"[{"ranges":[{"r0":0,"c0":3,"r1":3,"c1":3}],"regra":{"tipo":"lista","valores":["a"]},"quando":"recusar"}]"#.into());
    let partes = montar_partes(&w);
    let texto = |nome: &str| String::from_utf8(partes.iter().find(|(n, _)| n == nome).unwrap().1.clone()).unwrap();
    let estilos = texto("xl/styles.xml");
    assert!(estilos.contains("<dxfs count=\"1\"><dxf>"), "{estilos}");
    assert!(estilos.contains("<bgColor rgb=\"FFFFC7CE\"/>"));
    let aba = texto("xl/worksheets/sheet1.xml");
    let cf = aba.find("<conditionalFormatting").expect("conditionalFormatting");
    if let Some(dv) = aba.find("<dataValidations") {
        assert!(cf < dv);
    }
    assert!(aba.contains("dxfId=\"0\""));
}
