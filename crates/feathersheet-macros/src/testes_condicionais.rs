//! Casos de API de formatação condicional no host JavaScript Boa.
use super::*;
use feathersheet_core::operacoes::rules::{CellOp,Condition,ConditionalRule,ScalePointType};
fn regras(s:&feathersheet_core::planilha::sheet::Sheet)->Vec<ConditionalRule> {
    s.r.as_deref().map(|v|serde_json::from_str(v).unwrap()).unwrap_or_default()
}
#[test]
fn macro_cria_formatacao_de_valor_escala_e_barra_sem_alocar_celulas() {
    let mut wb=Workbook::new();
    let r=executar(&mut wb,0,r#"
        sheet.addConditionalFormat('A1:A10000000',{type:'cellValue',operator:'greaterThan',value:100,fill:'#F0A',bold:true});
        sheet.range('B1:B10').addColorScale({min:'#fff',max:'#22c55e',mid:'#ffd700'});
        sheet.addDataBar('C1:C10',{hideValue:true,color:'#123456'});
    "#);
    assert!(r.erro.is_none(),"{:?}",r.erro);
    let s=&wb.aba(0).unwrap().sheet;
    let lista=regras(s);
    assert_eq!(lista.len(),3);
    assert_eq!(lista[0].ranges[0].r1,9_999_999);
    assert!(matches!(&lista[0].condition,Condition::Cell(c)
        if c.op==CellOp::GreaterThan && c.values==["100"]
        && c.style.bg.as_deref()==Some("#ff00aa") && c.style.bold==Some(true)));
    assert!(matches!(&lista[1].condition,Condition::ColorScale(c)
        if c.mid.as_ref().map(|p|p.point_type)==Some(ScalePointType::Percentile)));
    assert!(matches!(&lista[2].condition,Condition::DataBar(c)
        if c.hide_value && c.color=="#123456"));
    assert_eq!(s.len(),0);
    assert!(wb.aba_mut(0).unwrap().sheet.undo());
    assert!(wb.aba(0).unwrap().sheet.r.is_none());
    assert!(wb.aba_mut(0).unwrap().sheet.redo());
    assert_eq!(regras(&wb.aba(0).unwrap().sheet).len(),3);
}
#[test]
fn limpar_condicionais_e_reverter_erro_na_macro() {
    let mut wb=Workbook::new();
    let r=executar(&mut wb,0,r#"
        sheet.addDataBar('A1:A10');
        sheet.range('C1:C10').addConditionalFormat({operator:'equal',value:12,color:255});
        sheet.clearConditionalFormats('A2');
    "#);
    assert!(r.erro.is_none(),"{:?}",r.erro);
    assert_eq!(regras(&wb.aba(0).unwrap().sheet).len(),1);
    assert_eq!(regras(&wb.aba(0).unwrap().sheet)[0].ranges[0].c0,2);
    let r=executar(&mut wb,0,r#"
        sheet.set('A1',42);
        sheet.addColorScale('D1',{min:'#fff',max:'#000'});
        sheet.addConditionalFormat('E1',{operator:'invalid',value:0});
    "#);
    assert!(r.erro.is_some());
    assert!(!r.alterou);
    assert_eq!(wb.aba(0).unwrap().sheet.get_raw(0,0),"");
    assert_eq!(regras(&wb.aba(0).unwrap().sheet).len(),1);
}
#[test]
fn limpar_intervalo_vazio_nao_marca_documento() {
    let mut wb=Workbook::new();
    let r=executar(&mut wb,0,"sheet.clearConditionalFormats('A1:Z10000000');");
    assert!(r.erro.is_none(),"{:?}",r.erro);
    assert!(!r.alterou);
}
