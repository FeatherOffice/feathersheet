//! Testes do contrato `fn-planilha.ts` para INDEX/MATCH no Boa.
//! As funções não alteram a planilha; todos os casos usam o motor Rust.
use super::*;

#[test]
fn macro_index_consulta_vetor_matriz_e_intervalo_real() {
    let mut wb=Workbook::new();
    wb.escrever_celula(0,0,0,"11".into());
    wb.escrever_celula(0,0,1,"22".into());
    wb.escrever_celula(0,1,0,"33".into());
    wb.escrever_celula(0,1,1,"44".into());
    let res=executar(&mut wb,0,r#"
        log(app.fn.index([10,20,30],2));
        log(app.fn.index([[1,2],[3,4]],2,2));
        log(app.fn.index(sheet.range('A1:B2').values(),2,1));
    "#);
    assert!(res.erro.is_none(), "{:?}",res.erro);
    assert_eq!(res.saida,vec!["20","4","33"]);
    assert!(!res.alterou);
}

#[test]
fn macro_match_exato_curingas_e_aproximacao_tipo_ts() {
    let mut wb=Workbook::new();
    let res=executar(&mut wb,0,r#"
        log(app.fn.match('ENERGIA',['água','energia','vento'],0));
        log(app.fn.match('ab?',['x','abc','abd'],0));
        log(app.fn.match(8,[1,9,7,6],1));
        log(app.fn.match(8,[1,9,7,10],-1));
        log(app.fn.match(5,['5',5],0));
    "#);
    assert!(res.erro.is_none(), "{:?}",res.erro);
    assert_eq!(res.saida,vec!["2","2","4","4","2"]);
    assert!(!res.alterou);
}

#[test]
fn macro_erros_de_busca_sao_valores_nao_excecoes() {
    let mut wb=Workbook::new();
    let res=executar(&mut wb,0,r#"
        const falta=app.fn.match('Z',['A','B']);
        const indice=app.fn.index([[10,20]],2,1);
        log(app.isError(falta)); log(falta.code);
        log(app.isError(indice)); log(indice.code);
        log(app.isError('#REF!')); log(String(indice));
    "#);
    assert!(res.erro.is_none(), "{:?}",res.erro);
    assert_eq!(res.saida,vec!["true","#N/A","true","#REF!","false","#REF!"]);
    assert!(!res.alterou);
}

#[test]
fn macro_buscas_nao_aceitam_indices_fracionarios() {
    let mut wb=Workbook::new();
    let res=executar(&mut wb,0,r#"
        log(app.fn.index([2,4,6],1.5).code);
        log(app.fn.index([2,4,6],0).code);
        log(app.fn.index([2,4,6],4).code);
    "#);
    assert!(res.erro.is_none(), "{:?}",res.erro);
    assert_eq!(res.saida,vec!["#REF!","#REF!","#REF!"]);
    assert!(!res.alterou);
}

#[test]
fn macro_busca_erro_no_argumento_reverte_set_anterior() {
    let mut wb=Workbook::new();
    let res=executar(&mut wb,0,
        "sheet.set('A1',123); app.fn.match('x',['x'],{invalido:true});");
    assert!(res.erro.is_some());
    assert!(!res.alterou);
    assert_eq!(wb.aba(0).unwrap().sheet.get_raw(0,0),"");
}

#[test]
fn macro_vlookup_e_hlookup_compartilham_pesquisa_no_core() {
    let mut wb=Workbook::new();
    let r=executar(&mut wb,0,r#"
        log(app.fn.vlookup('B',[['A',10],['B',20],['C',30]],2));
        log(app.fn.hlookup(5,[[2,5,10],[20,50,100]],2));
        log(app.fn.vlookup(6,[[2,20],[5,50],[10,100]],2,false));
        log(app.isError(app.fn.hlookup(99,[[1,2],[3,4]],2)));
    "#);
    assert!(r.erro.is_none(),"{:?}",r.erro);
    assert_eq!(r.saida,vec!["20","50","50","true"]);
    assert!(!r.alterou);
}

