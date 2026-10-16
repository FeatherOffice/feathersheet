//! Regressões de set/clearDataValidation pela API JavaScript Boa.
use super::*;
use feathersheet_core::operacoes::validation::{NumeroOp, Regra, Validacao, QuandoValidacao};

fn regras(sheet:&feathersheet_core::planilha::sheet::Sheet)->Vec<Validacao> {
    serde_json::from_str(sheet.va.as_deref().unwrap_or("[]")).unwrap()
}

#[test]
fn macro_aplica_validacao_de_lista_e_numero_sem_criar_celulas() {
    let mut wb=Workbook::new();
    let r=executar(&mut wb,0,r#"
        sheet.setDataValidation('A1:A10000000',{type:'list',values:['Sim','Não']});
        sheet.range('B1:B10').setDataValidation({
            type:'number',operator:'between',min:10,max:20,
            mode:'warn',errorMessage:'Escolha valor de 10 a 20'
        });
    "#);
    assert!(r.erro.is_none(),"{:?}",r.erro);
    assert!(r.alterou);
    let s=&wb.aba(0).unwrap().sheet;
    let rs=regras(s);
    assert_eq!(rs.len(),2);
    assert_eq!(rs[0].ranges[0].r1,9_999_999);
    assert!(matches!(&rs[0].regra,Regra::Lista{valores} if valores==&vec!["Sim".to_string(),"Não".to_string()]));
    assert!(matches!(&rs[1].regra,Regra::Numero{op:NumeroOp::Entre,a:10.0,b:Some(20.0),..}));
    assert_eq!(rs[1].quando,QuandoValidacao::Avisar);
    assert_eq!(rs[1].mensagem.as_deref(),Some("Escolha valor de 10 a 20"));
    assert_eq!(s.len(),0,"metadados não alocam células na grade");
    assert!(wb.aba_mut(0).unwrap().sheet.undo());
    assert!(wb.aba(0).unwrap().sheet.va.is_none());
}

#[test]
fn macro_limpa_apenas_validacoes_intersectadas() {
    let mut wb=Workbook::new();
    let r=executar(&mut wb,0,r#"
        sheet.setDataValidation('A1:A5',{type:'checkbox'});
        sheet.setDataValidation('C1:C5',{type:'text',operator:'contains',value:'abc'});
        sheet.clearDataValidation('A2');
    "#);
    assert!(r.erro.is_none(),"{:?}",r.erro);
    let rs=regras(&wb.aba(0).unwrap().sheet);
    assert_eq!(rs.len(),1);
    assert!(matches!(&rs[0].regra,Regra::Texto{..}));
    assert_eq!(rs[0].ranges[0].c0,2);
    assert!(wb.aba_mut(0).unwrap().sheet.undo());
    assert!(wb.aba(0).unwrap().sheet.va.is_none());
}

#[test]
fn macro_validacao_invalida_reverte_valores_e_metadados() {
    let mut wb=Workbook::new();
    let r=executar(&mut wb,0,r#"
        sheet.set('A1',77);
        sheet.setDataValidation('B1',{type:'list',values:['A','B']});
        sheet.setDataValidation('C1',{type:'number',operator:'between',min:3});
    "#);
    assert!(r.erro.is_some());
    assert!(!r.alterou);
    let s=&wb.aba(0).unwrap().sheet;
    assert_eq!(s.get_raw(0,0),"");
    assert!(s.va.is_none());
}

#[test]
fn macro_aceita_validacao_de_intervalo_e_datas_sem_recriar_motor() {
    let mut wb=Workbook::new();
    let r=executar(&mut wb,0,r#"
        sheet.setDataValidation('E1:E10',{type:'list',source:'A1:A10'});
        sheet.setDataValidation('F1:F10',{type:'date',operator:'after',value:'2026-01-01'});
    "#);
    assert!(r.erro.is_none(),"{:?}",r.erro);
    let rs=regras(&wb.aba(0).unwrap().sheet);
    assert_eq!(rs.len(),2);
    assert!(matches!(rs[0].regra,Regra::Intervalo{..}));
    assert!(matches!(rs[1].regra,Regra::Data{..}));
}

#[test]
fn macro_nao_marca_documento_quando_limpeza_nao_altera_regras() {
    let mut wb=Workbook::new();
    let r=executar(&mut wb,0,"sheet.clearDataValidation('A1:Z50');");
    assert!(r.erro.is_none(),"{:?}",r.erro);
    assert!(!r.alterou);
    assert!(wb.aba(0).unwrap().sheet.va.is_none());
}

#[test]
fn validacao_por_formula_e_operadores_do_excel() {
    use feathersheet_core::operacoes::validation::{Regra, Validacao, NumeroOp};
    let mut wb=Workbook::new();
    wb.criar_aba("Dados");
    let res=executar(&mut wb,0,concat!(
        "sheet.setDataValidation('A1', {type:'list', source:'=INDIRECT(B1)'});",
        "sheet.setDataValidation('A2', {type:'list', source:'Dados!A1:A9'});",
        "sheet.setDataValidation('A3', {type:'list', source:'=$C$1:$C$4'});",
        "sheet.setDataValidation('A4', {type:'number', operator:'greaterThanOrEqual', value:5, integer:true});"
    ));
    assert!(res.erro.is_none(),"{:?}",res.erro);
    let regras:Vec<Validacao>=serde_json::from_str(wb.aba(0).unwrap().sheet.va.as_deref().unwrap()).unwrap();
    let r:Vec<_>=regras.iter().map(|v|v.regra.clone()).collect();
    assert_eq!(r[0],Regra::Formula{formula:"=INDIRECT(B1)".into()});
    assert_eq!(r[1],Regra::Formula{formula:"=Dados!A1:A9".into()});
    assert!(matches!(r[2],Regra::Intervalo{..}));
    assert_eq!(r[3],Regra::Numero{op:NumeroOp::MaiorIgual,a:5.0,b:None,inteiro:true});
}
