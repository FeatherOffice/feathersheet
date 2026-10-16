//! Regressões de API de formatação do antigo api-formato.ts em Boa/Rust.
//! Escritos para cargo test local; GitHub Actions deliberadamente desativado.
use super::*;
use feathersheet_core::celulas::{
    estilo_parcial::{Campo, EstiloParcial},
    refs::Rect,
};

#[test]
fn formato_e_borda_do_mesmo_intervalo_preservam_valores_e_desfazem_uma_vez() {
    let mut wb = Workbook::new();
    wb.escrever_celula(0,0,0,"100".into());
    wb.escrever_celula(0,0,1,"200".into());
    let res=executar(&mut wb,0,
        "sheet.setBold('A1:B1'); sheet.setBackground('A1:B1','#09f'); sheet.setOutline('A1:B1',{color:'#f00'});");
    assert!(res.erro.is_none(),"{:?}",res.erro);
    assert!(res.alterou);
    let s=&wb.aba(0).unwrap().sheet;
    let e=s.estilo_resolvido(0,0).unwrap();
    assert!(e.bold);
    assert_eq!(e.bg, Some("#0099ff".into()));
    let b=e.borders.unwrap();
    assert!(b.top && b.left && b.bottom);
    assert_eq!(b.color, Some("#ff0000".into()));
    assert_eq!(s.get_raw(0,0),"100");
    assert_eq!(s.get_raw(0,1),"200");
    assert!(wb.aba_mut(0).unwrap().sheet.undo());
    let s=&wb.aba(0).unwrap().sheet;
    assert_eq!(s.get_raw(0,0),"100");
    assert!(s.estilo_resolvido(0,0).is_none());
    assert!(s.estilo_resolvido(0,1).is_none());
}

#[test]
fn macro_bordas_acumulam_em_chamadas_sucessivas() {
    let mut wb = Workbook::new();
    let res=executar(&mut wb,0,
        "sheet.setBorder('A1:B2',{top:true}); sheet.setBorder('A1:B2',{insideV:true,bottom:true});");
    assert!(res.erro.is_none(),"{:?}",res.erro);
    let s=&wb.aba(0).unwrap().sheet;
    let b=s.estilo_resolvido(0,0).unwrap().borders.unwrap();
    assert!(b.top);
    assert!(b.right);
    assert!(!b.bottom);
    let fim=s.estilo_resolvido(1,0).unwrap().borders.unwrap();
    assert!(fim.bottom);
}

#[test]
fn formato_invalido_apos_outros_comandos_reverte_sem_estado_parcial() {
    let mut wb = Workbook::new();
    wb.escrever_celula(0,0,0,"valor".into());
    let res=executar(&mut wb,0,
        "sheet.setBold('A1'); sheet.setBackground('A1','#f0a'); sheet.setOutline('A1',{width:2});");
    assert!(res.erro.is_some());
    assert!(!res.alterou);
    assert_eq!(wb.aba(0).unwrap().sheet.get_raw(0,0),"valor");
    assert!(wb.aba(0).unwrap().sheet.estilo_resolvido(0,0).is_none());
}

#[test]
fn estilo_herdado_da_coluna_nao_vira_estilo_direto_ao_definir_italico() {
    let mut wb=Workbook::new();
    let mut estilo=EstiloParcial::new();
    estilo.insert("bold".into(),Campo::Logico(true));
    wb.aba_mut(0).unwrap().sheet
        .set_style(&Rect::new(0,0,9_999_999,0),&estilo);
    let res=executar(&mut wb,0,"sheet.setItalic('A1');");
    assert!(res.erro.is_none(),"{:?}",res.erro);
    let s=&wb.aba(0).unwrap().sheet;
    assert!(s.estilo_resolvido(0,0).unwrap().bold);
    assert!(s.estilo_resolvido(0,0).unwrap().italic);
    assert!(wb.aba_mut(0).unwrap().sheet.undo());
    let s=&wb.aba(0).unwrap().sheet;
    assert!(s.estilo_resolvido(0,0).unwrap().bold);
    assert!(!s.estilo_resolvido(0,0).unwrap().italic);
}
