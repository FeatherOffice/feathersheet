//! Regressões da parte puramente numérica de api-aplicacao.ts e
//! getter da pasta api-pasta.ts. Todas as cores são calculadas no Rust.
use super::*;

#[test]
fn app_calculate_reinvalida_formulas_sem_edicao_ou_undo() {
    let mut wb=Workbook::new();
    wb.escrever_celula(0,0,0,"10".into());
    wb.escrever_celula(0,0,1,"=A1*2".into());
    let r=executar(&mut wb,0,
        "log(sheet.get('B1'));app.calculate();log(sheet.get('B1'));");
    assert!(r.erro.is_none(),"{:?}",r.erro);
    assert_eq!(r.saida,vec!["20","20"]);
    assert!(!r.alterou,"invalidar cache não altera o documento");
    assert_eq!(wb.aba(0).unwrap().sheet.get_raw(0,1),"=A1*2");
}

#[test]
fn status_da_macro_vai_para_a_gui_sem_alterar_planilha() {
    let mut pasta=Workbook::new();
    let r=executar(&mut pasta,0,
        "app.status('Importando arquivos'); log('ok'); app.status('Concluído: 100%');");
    assert!(r.erro.is_none(),"{:?}",r.erro);
    assert_eq!(r.status.as_deref(),Some("Concluído: 100%"));
    assert_eq!(r.saida,vec!["ok"]);
    assert!(!r.alterou);
}

#[test]
fn status_em_macro_com_erro_nao_e_publicado() {
    let mut pasta=Workbook::new();
    let r=executar(&mut pasta,0,
        "app.status('Prévia'); sheet.set('A1',99); throw new Error('falhou');");
    assert!(r.erro.is_some());
    assert_eq!(r.status,None);
    assert!(!r.alterou);
    assert_eq!(pasta.aba(0).unwrap().sheet.get_raw(0,0),"");
}

#[test]
fn app_error_valida_codigos_no_rust_e_retorna_erro_js() {
    let mut pasta=Workbook::new();
    let r=executar(&mut pasta,0,r#"
        const e=app.error('#N/A','não encontrado');
        log(app.isError(e)); log(e.code); log(e.detail);
        log(app.isError('#N/A'));
    "#);
    assert!(r.erro.is_none(),"{:?}",r.erro);
    assert_eq!(r.saida,vec!["true","#N/A","não encontrado","false"]);
    assert!(!r.alterou);

    let inv=executar(&mut pasta,0,
        "sheet.set('A1',123);app.error('#ERRO-INVENTADO');");
    assert!(inv.erro.is_some());
    assert!(!inv.alterou);
    assert_eq!(pasta.aba(0).unwrap().sheet.get_raw(0,0),"");
}

#[test]
fn rgb_e_hex_vba_usam_mesma_conversao_do_rust() {
    let mut wb=Workbook::new();
    let r=executar(&mut wb,0,
        "log(app.rgb(255,0,128));log(app.vbaColorToHex(app.rgb(255,0,128)));");
    assert!(r.erro.is_none(), "{:?}",r.erro);
    assert_eq!(r.saida,vec!["8388863","#ff0080"]);
    assert!(!r.alterou);
}

#[test]
fn batch_preserva_retorno_e_um_unico_desfazer() {
    let mut wb=Workbook::new();
    let r=executar(&mut wb,0,
        "const retorno=app.batch(()=>{sheet.set('A1',10);sheet.set('B1',20);return 7;});log(retorno);");
    assert!(r.erro.is_none(),"{:?}",r.erro);
    assert_eq!(wb.aba(0).unwrap().sheet.get_raw(0,0),"10");
    assert_eq!(wb.aba(0).unwrap().sheet.get_raw(0,1),"20");
    assert!(r.saida.iter().any(|x|x.contains('7')));
    assert!(wb.aba_mut(0).unwrap().sheet.undo());
    assert_eq!(wb.aba(0).unwrap().sheet.get_raw(0,0),"");
    assert_eq!(wb.aba(0).unwrap().sheet.get_raw(0,1),"");
}

#[test]
fn canais_invalidos_revertem_edicoes_anteriores() {
    let mut wb=Workbook::new();
    let r=executar(&mut wb,0,
        "sheet.set('A1',100); app.rgb(256,0,0);");
    assert!(r.erro.is_some());
    assert!(!r.alterou);
    assert_eq!(wb.aba(0).unwrap().sheet.get_raw(0,0),"");
}

#[test]
fn contagem_de_abas_reaproveita_lista_do_workbook() {
    let mut wb=Workbook::new();
    wb.criar_aba("Dados");
    let r=executar(&mut wb,0,
        "log(wb.sheetCount);log(wb.sheetNames().length);");
    assert!(r.erro.is_none(),"{:?}",r.erro);
    assert_eq!(r.saida,vec!["2","2"]);
    assert!(!r.alterou);
}

#[test]
fn worksheetfunction_agregacoes_operam_no_rust() {
    let mut wb = Workbook::new();
    let r = executar(&mut wb, 0, r#"
        log(app.fn.sum([[1,2],[3,'4'],[null,true]]));
        log(app.fn.count([[1,2],[3,'4'],[null,true]]));
        log(app.fn.countA([0,false,'',null,{},[]]));
        log(app.fn.max([-20,-4,-8]));
        log(app.fn.min(['x',null]));
    "#);
    assert!(r.erro.is_none(), "{:?}", r.erro);
    assert_eq!(r.saida,vec!["6","3","3","-4","0"]);
    assert!(!r.alterou);
}

#[test]
fn worksheetfunction_buscas_rust_preservam_api_javascript() {
    let mut wb=Workbook::new();
    let resultado=executar(&mut wb,0,r#"
        log(app.fn.vlookup('b',[['A',10],['B',20]],2));
        log(app.fn.hlookup(20,[[10,20],[100,200]],2));
        log(app.fn.match('ab*',['uva','ABACAXI','banana']));
        log(app.fn.index([[1,2],[3,4]],2,1));
        log(app.fn.index([7,8,9],2));
        log(app.isError(app.fn.vlookup('x',[['a',1]],2)));
        log(app.fn.vlookup('x',[['a',1]],2).code);
        log(app.fn.index([1,2],9).code);
    "#);
    assert!(resultado.erro.is_none(),"{:?}",resultado.erro);
    assert_eq!(resultado.saida,vec!["20","200","2","3","8","true","#N/A","#REF!"]);
    assert!(!resultado.alterou);
}

#[test]
fn worksheetfunction_buscas_com_erro_revertem_transacao() {
    let mut wb=Workbook::new();
    let resultado=executar(&mut wb,0,
        "sheet.set('A1',2);app.fn.index([1],0);throw new Error('falhou');");
    assert!(resultado.erro.is_some());
    assert!(!resultado.alterou);
    assert_eq!(wb.aba(0).unwrap().sheet.get_raw(0,0),"");
}

#[test]
fn countif_e_sumif_sao_executados_pelo_predicado_do_core() {
    let mut wb=Workbook::new();
    let r=executar(&mut wb,0,
        "log(app.fn.countIf([[2,6],[9,null],['texto',20]],'>5'));         log(app.fn.sumIf([2,6,9],'>5',[100,200,300]));         log(app.fn.countIf(['Abacaxi','ABACATE','uva'],'aba*'));");
    assert!(r.erro.is_none(),"{:?}",r.erro);
    assert_eq!(r.saida,vec!["3","500","2"]);
    assert!(!r.alterou,"WorksheetFunction é somente leitura");
}

#[test]
fn countif_consulta_valores_do_intervalo_sem_duplicar_grade() {
    let mut wb=Workbook::new();
    wb.escrever_celula(0,0,0,"2".into());
    wb.escrever_celula(0,1,0,"6".into());
    wb.escrever_celula(0,2,0,"8".into());
    let r=executar(&mut wb,0,
        "log(app.fn.countIf(sheet.range('A1:A3').values(),'>5'));         log(app.fn.sumIf(sheet.range('A1:A3').values(),'>5'));");
    assert!(r.erro.is_none(),"{:?}",r.erro);
    assert_eq!(r.saida,vec!["2","14"]);
    assert!(!r.alterou);
}

#[test]
fn wb_sheet_resolve_nome_sem_diferenciar_maiusculas() {
    let mut wb=Workbook::new();
    wb.criar_aba("Dados de Energia");
    let r=executar(&mut wb,0,
        "wb.sheet('  dAdOs De EnErGiA  ').set('A1',42);         log(wb.sheet(2).get('A1'));");
    assert!(r.erro.is_none(),"{:?}",r.erro);
    assert_eq!(wb.aba(1).unwrap().sheet.get_raw(0,0),"42");
    assert_eq!(r.saida,vec!["42"]);
    assert!(wb.aba_mut(1).unwrap().sheet.undo());
    assert_eq!(wb.aba(1).unwrap().sheet.get_raw(0,0),"");
}

#[test]
fn wb_sheet_fora_da_faixa_nao_altera_planilha() {
    let mut wb=Workbook::new();
    for js in [
        "wb.sheet('inexistente').set('A1',1);",
        "wb.sheet(0).set('A1',1);",
        "wb.sheet(1.5).set('A1',1);",
    ] {
        let r=executar(&mut wb,0,js);
        assert!(r.erro.is_some(),"código deveria falhar: {js}");
        assert!(!r.alterou);
    }
    assert_eq!(wb.aba(0).unwrap().sheet.len(),0);
}

#[test]
fn criterio_invalido_apos_set_nao_deixa_mudanca() {
    let mut wb=Workbook::new();
    let r=executar(&mut wb,0,
        "sheet.set('A1',123);app.fn.countIf([1,2],{op:'>'});");
    assert!(r.erro.is_some());
    assert!(!r.alterou);
    assert_eq!(wb.aba(0).unwrap().sheet.get_raw(0,0),"");
}

#[test]
fn averageif_mediana_e_media_reutilizam_calculo_do_core() {
    let mut wb=Workbook::new();
    let res=executar(&mut wb,0,r#"
        log(app.fn.average([[1,2],['texto',3]]));
        log(app.fn.median([10,2,7,5]));
        log(app.fn.averageIf([2,6,9],'>5',[100,200,300]));
        log(app.fn.averageIf([2,6,9],'>5'));
    "#);
    assert!(res.erro.is_none(),"{:?}",res.erro);
    assert_eq!(res.saida,vec!["2","6","250","7.5"]);
    assert!(!res.alterou);
}

#[test]
fn erros_de_medias_sao_objetos_no_javascript() {
    let mut wb=Workbook::new();
    let res=executar(&mut wb,0,r#"
        const vazio=app.fn.average([]);
        const mediana=app.fn.median([]);
        const semMedia=app.fn.averageIf([1,2,3],'>100');
        log(app.isError(vazio)); log(vazio.code); log(String(vazio));
        log(app.isError(mediana)); log(mediana.code);
        log(app.isError(semMedia)); log(semMedia.code);
        log(app.isError('#DIV/0!'));
    "#);
    assert!(res.erro.is_none(),"{:?}",res.erro);
    assert_eq!(res.saida,vec!["true","#DIV/0!","#DIV/0!","true","#NUM!","true","#DIV/0!","false"]);
    assert!(!res.alterou);
}

#[test]
fn erros_de_formula_diferem_de_texto_literal_com_codigo() {
    let mut wb=Workbook::new();
    wb.escrever_celula(0,0,0,"=1/0".into());
    wb.escrever_celula(0,1,0,"#DIV/0!".into());
    let res=executar(&mut wb,0,r#"
        const erro=sheet.get('A1');
        log(app.isError(erro)); log(erro.code);
        log(app.isError(sheet.get('A2')));
        log(app.isError(sheet.range('A1:A2').values()[0][0]));
    "#);
    assert!(res.erro.is_none(),"{:?}",res.erro);
    assert_eq!(res.saida,vec!["true","#DIV/0!","false","true"]);
    assert!(!res.alterou);
}

#[test]
fn media_com_criterio_invalido_desfaz_escrita_previa() {
    let mut wb=Workbook::new();
    let r=executar(&mut wb,0,
        "sheet.set('A1',42);app.fn.averageIf([1,2,3],{});");
    assert!(r.erro.is_some());
    assert!(!r.alterou);
    assert_eq!(wb.aba(0).unwrap().sheet.get_raw(0,0),"");
}
