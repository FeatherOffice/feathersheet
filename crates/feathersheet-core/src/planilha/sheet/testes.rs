use super::*;

#[test]
fn cascata_de_sujeira_visita_cada_formula_uma_vez() {
    // Cada linha soma as duas de cima (intervalo: fica no grafo, não na
    // coluna empacotada). São fib(60) caminhos até a última — a cascata
    // antiga seguia todos e travou o .xlsx do usuário (12 abas, >4 GB).
    let mut s = Sheet::new("Aba1");
    s.set_cell_raw(0, 0, "1".into(), None);
    s.set_cell_raw(1, 0, "1".into(), None);
    for r in 2..60u32 {
        s.set_cell_raw(r, 0, format!("=SOMA(A{}:A{})", r - 1, r), None);
    }
    let inicio = std::time::Instant::now();
    s.set_cell_raw(0, 0, "2".into(), None);
    assert!(inicio.elapsed().as_secs() < 2, "cascata exponencial");
    let (mut a, mut b) = (2.0f64, 1.0f64);
    for _ in 2..60 { (a, b) = (b, a + b); }
    assert_eq!(s.get_value(59, 0), Value::Number(b));
}

#[test]
fn resumo_selecao_conta_numeros_textos_formula_e_ignora_erros_e_linhas_ocultas() {
    let mut s=Sheet::new("Aba1");
    s.set_cell_raw(0,0,"5".into(),None);
    s.set_cell_raw(0,1,"Nome".into(),None);
    s.set_cell_raw(1,0,"100".into(),None);
    s.set_cell_raw(2,0,"=A1*2".into(),None);
    s.set_cell_raw(3,0,"=1/0".into(),None);
    s.row_heights.insert(1,0);
    let a=s.contar_selecao(&Rect::new(0,0,3,1),None);
    assert_eq!(a.count,3);
    assert_eq!(a.numbers,2);
    assert_eq!(a.sum,15.);
    assert_eq!(a.media(),Some(7.5));
    assert_eq!(a.min,5.);
    assert_eq!(a.max,10.);
    assert_eq!(s.contar_selecao(&Rect::new(0,0,0,1),None).count,2);
}
#[test]
fn resumo_da_coluna_inteira_recorta_ate_dados_usados() {
    let mut s=Sheet::new("Aba1");
    s.set_cell_raw(0,0,"7".into(),None);
    s.set_cell_raw(999_999,0,"13".into(),None);
    let total=s.contar_selecao(&Rect::new(0,0,MAX_ROWS-1,0),None);
    assert_eq!(total.count,2);
    assert_eq!(total.numbers,2);
    assert_eq!(total.sum,20.);
    let vazia=s.contar_selecao(&Rect::new(0,2,MAX_ROWS-1,2),None);
    assert_eq!(vazia.count,0);
    assert_eq!(vazia.media(),None);
}
#[test]
fn resumo_une_lotes_sem_recontar_celulas() {
    let mut a=ContagemSelecao::default();
    let mut b=ContagemSelecao::default();
    a.registrar(&Value::Number(3.));
    b.registrar(&Value::Text("texto".into()));
    b.registrar(&Value::Number(9.));
    a.somar_lote(&b);
    assert_eq!((a.count,a.numbers,a.sum,a.min,a.max),(3,2,12.,3.,9.));
}

#[test]
fn insere_e_remove_linhas_em_rust() {
    let mut sheet = Sheet::new("Aba1");
    sheet.set_cell_raw(0, 0, "100".into(), None);
    sheet.set_cell_raw(5, 0, "500".into(), None);

    sheet.insert_rows(2, 2);
    assert_eq!(sheet.get_raw(0, 0), "100");
    assert_eq!(sheet.get_raw(5, 0), "");
    assert_eq!(sheet.get_raw(7, 0), "500");

    sheet.delete_rows(2, 2);
    assert_eq!(sheet.get_raw(5, 0), "500");
}

#[test]
fn insere_e_remove_colunas_em_rust() {
    let mut sheet = Sheet::new("Aba1");
    sheet.set_cell_raw(0, 1, "colB".into(), None);

    sheet.insert_cols(1, 2);
    assert_eq!(sheet.get_raw(0, 1), "");
    assert_eq!(sheet.get_raw(0, 3), "colB");

    sheet.delete_cols(1, 2);
    assert_eq!(sheet.get_raw(0, 1), "colB");
}

#[test]
fn literal_guarda_numero_texto_e_booleano_de_verdade() {
    // O que o `HashMap<u64, f64>` antigo não conseguia: os três tipos,
    // sem duplicar em `cells`.
    let mut sheet = Sheet::new("Aba1");
    sheet.set_cell_raw(0, 0, "42".into(), None);
    sheet.set_cell_raw(1, 0, "olá".into(), None);
    sheet.set_cell_raw(2, 0, "true".into(), None);

    assert_eq!(sheet.get_value(0, 0), Value::Number(42.0));
    assert_eq!(sheet.get_value(1, 0), Value::Text("olá".to_string()));
    assert_eq!(sheet.get_value(2, 0), Value::Bool(true));
    // Nenhum dos três é fórmula — não deveriam estar em `cells`.
    assert!(sheet.cells.is_empty());
}

#[test]
fn formula_continua_em_cells_literal_nao() {
    let mut sheet = Sheet::new("Aba1");
    sheet.set_cell_raw(0, 0, "=A1+1".into(), None);
    sheet.set_cell_raw(1, 0, "10".into(), None);
    assert!(sheet.get_raw(0, 0).starts_with('='));
    assert!(!sheet.get_raw(1, 0).starts_with('='));
}

#[test]
fn trocar_de_formula_para_literal_limpa_o_lado_antigo() {
    let mut sheet = Sheet::new("Aba1");
    sheet.set_cell_raw(0, 0, "=A1".into(), None);
    assert!(sheet.get_raw(0, 0).starts_with('='));
    sheet.set_cell_raw(0, 0, "5".into(), None);
    assert!(!sheet.get_raw(0, 0).starts_with('='));
    assert_eq!(sheet.get_value(0, 0), Value::Number(5.0));

    sheet.set_cell_raw(0, 0, "=A2".into(), None);
    assert!(sheet.get_raw(0, 0).starts_with('='));
    assert_eq!(sheet.get_value(0, 0), Value::Empty); // fórmula nunca calculada
}

#[test]
fn estilo_de_literal_sobrevive_e_some_com_a_celula() {
    let mut sheet = Sheet::new("Aba1");
    sheet.set_cell_raw(0, 0, "5".into(), Some(3));
    assert_eq!(sheet.get_style_idx(0, 0), Some(3));
    sheet.set_cell_raw(0, 0, String::new(), None);
    assert_eq!(sheet.get_style_idx(0, 0), None);
}

#[test]
fn insere_linha_desloca_literal_de_todos_os_tipos() {
    let mut sheet = Sheet::new("Aba1");
    sheet.set_cell_raw(5, 0, "42".into(), None);
    sheet.set_cell_raw(5, 1, "texto".into(), None);
    sheet.set_cell_raw(5, 2, "true".into(), None);

    sheet.insert_rows(2, 3);

    assert_eq!(sheet.get_value(8, 0), Value::Number(42.0));
    assert_eq!(sheet.get_value(8, 1), Value::Text("texto".to_string()));
    assert_eq!(sheet.get_value(8, 2), Value::Bool(true));
    assert_eq!(sheet.get_value(5, 0), Value::Empty);
}

#[test]
fn len_e_is_empty_contam_literal_e_formula() {
    let mut sheet = Sheet::new("Aba1");
    assert!(sheet.is_empty());
    sheet.set_cell_raw(0, 0, "1".into(), None);
    sheet.set_cell_raw(0, 1, "=A1".into(), None);
    assert!(!sheet.is_empty());
    assert_eq!(sheet.len(), 2);
}

// ---- Recálculo: o que este arquivo ganhou nesta sessão -----------------

#[test]
fn formula_simples_calcula_de_verdade() {
    let mut sheet = Sheet::new("Aba1");
    sheet.set_cell_raw(0, 0, "10".into(), None); // A1
    sheet.set_cell_raw(0, 1, "=A1+5".into(), None); // B1
    assert_eq!(sheet.get_value(0, 1), Value::Number(15.0));
}

#[test]
fn mudar_precedente_recalcula_o_dependente() {
    let mut sheet = Sheet::new("Aba1");
    sheet.set_cell_raw(0, 0, "10".into(), None); // A1
    sheet.set_cell_raw(0, 1, "=A1*2".into(), None); // B1
    assert_eq!(sheet.get_value(0, 1), Value::Number(20.0));

    sheet.set_cell_raw(0, 0, "100".into(), None); // A1 muda
    assert_eq!(sheet.get_value(0, 1), Value::Number(200.0)); // B1 recalculou sozinho
}

#[test]
fn cadeia_de_dependentes_recalcula_em_cascata() {
    let mut sheet = Sheet::new("Aba1");
    sheet.set_cell_raw(0, 0, "1".into(), None); // A1
    sheet.set_cell_raw(0, 1, "=A1+1".into(), None); // B1 = 2
    sheet.set_cell_raw(0, 2, "=B1+1".into(), None); // C1 = 3
    assert_eq!(sheet.get_value(0, 2), Value::Number(3.0));

    sheet.set_cell_raw(0, 0, "10".into(), None);
    assert_eq!(sheet.get_value(0, 1), Value::Number(11.0));
    assert_eq!(sheet.get_value(0, 2), Value::Number(12.0));
}

#[test]
fn referencia_circular_direta_vira_erro_em_vez_de_estourar_a_pilha() {
    let mut sheet = Sheet::new("Aba1");
    sheet.set_cell_raw(0, 0, "=A2".into(), None); // A1 = A2
    sheet.set_cell_raw(1, 0, "=A1".into(), None); // A2 = A1

    assert_eq!(sheet.get_value(0, 0), Value::Error(CellError::new(ErrorCode::Circular)));
}

#[test]
fn soma_de_intervalo_enxerga_literal_e_formula_juntos() {
    let mut sheet = Sheet::new("Aba1");
    sheet.set_cell_raw(0, 0, "1".into(), None); // A1 = 1 (literal)
    sheet.set_cell_raw(1, 0, "=A1+1".into(), None); // A2 = 2 (fórmula)
    sheet.set_cell_raw(2, 0, "=SOMA(A1:A2)".into(), None); // A3

    assert_eq!(sheet.get_value(2, 0), Value::Number(3.0));
}

#[test]
fn leitura_emprestada_de_texto_nao_aloca_e_preserva_raw_original() {
    let mut sheet=Sheet::new("A");
    sheet.set_cell_raw(0,0,"track_id,artist,album".into(),None);
    sheet.set_cell_raw(1,0,"0012".into(),None);
    sheet.set_cell_raw(2,0,"123".into(),None);
    sheet.set_cell_raw(3,0,"=A3+1".into(),None);
    let texto=sheet.texto_cru_divisao(0,0).unwrap();
    assert_eq!(texto,"track_id,artist,album");
    // "0012" vira o número 12, como no Excel (entrada portada em 12/10). Número e
    // fórmula não têm texto emprestável: quem chama cai no `get_raw`.
    assert_eq!(sheet.texto_cru_divisao(1,0),None);
    assert_eq!(sheet.get_raw(1,0),"12");
    assert_eq!(sheet.texto_cru_divisao(2,0),None);
    assert_eq!(sheet.texto_cru_divisao(3,0),None);
    assert_eq!(sheet.get_raw(3,0),"=A3+1");
    assert_eq!(sheet.get_raw(2,0),"123");
}

#[test]
fn invalidacao_total_reusa_motor_preguicoso_e_nao_apaga_conteudo() {
    let mut sheet=Sheet::new("Dados");
    sheet.set_cell_raw(0,0,"2".into(),None);
    sheet.set_cell_raw(0,1,"=A1+1".into(),None);
    sheet.set_cell_raw(1,1,"=SOMA(A1:A1)+4".into(),None);
    assert_eq!(sheet.get_value(0,1),Value::Number(3.0));
    assert_eq!(sheet.get_value(1,1),Value::Number(6.0));
    assert!(!sheet.calc.borrow().cache.is_empty());
    sheet.invalidar_todos_calculos();
    assert!(sheet.calc.borrow().cache.is_empty());
    assert_eq!(sheet.get_raw(0,1),"=A1+1");
    assert_eq!(sheet.get_value(0,1),Value::Number(3.0));
    assert_eq!(sheet.get_value(1,1),Value::Number(6.0));
}

#[test]
fn um_lote_de_literais_sem_formulas_nao_acumula_cache_sujo() {
    let mut sheet = Sheet::new("Dados");
    for r in 0..25_000 {
        sheet.set_cell_raw(r, 0, format!("Ana;{r};ES"), None);
    }
    assert_eq!(sheet.get_raw(24_999, 0), "Ana;24999;ES");
    assert!(sheet.calc.borrow().dirty.is_empty(),
        "25 mil literais sem dependentes não devem ocupar o cache de cálculo");
    sheet.set_cell_raw(0, 0, "novo".into(), None);
    assert!(sheet.calc.borrow().dirty.is_empty());
}

#[test]
fn com_formula_ainda_nao_acumula_literais_no_cache_sujo() {
    let mut sheet = Sheet::new("Dados");
    sheet.set_cell_raw(0, 1, "=A1*2".into(), None);
    for r in 0..20_000 {
        sheet.set_cell_raw(r, 0, (r + 1).to_string(), None);
    }
    // Apenas a fórmula B1 depende de A1. Nem B1 nem outros 19.999
    // literais podem fazer o HashSet dirty crescer com o lote.
    assert!(sheet.calc.borrow().dirty.len() <= 1);
    assert_eq!(sheet.get_value(0, 1), Value::Number(2.0));
    sheet.set_cell_raw(0, 0, "10".into(), None);
    assert_eq!(sheet.get_value(0, 1), Value::Number(20.0));
}

#[test]
fn apagar_precedente_suja_o_dependente() {
    let mut sheet = Sheet::new("Aba1");
    sheet.set_cell_raw(0, 0, "5".into(), None); // A1
    sheet.set_cell_raw(0, 1, "=A1".into(), None); // B1
    assert_eq!(sheet.get_value(0, 1), Value::Number(5.0));

    sheet.set_cell_raw(0, 0, String::new(), None); // apaga A1
    assert_eq!(sheet.get_value(0, 1), Value::Empty);
}

#[test]
fn inserir_linha_desloca_formula_e_mantem_recalculo_ligado() {
    let mut sheet = Sheet::new("Aba1");
    sheet.set_cell_raw(0, 0, "7".into(), None); // A1
    sheet.set_cell_raw(0, 1, "=A1".into(), None); // B1 = 7, lê A1

    sheet.insert_rows(0, 1); // A1 vira A2, B1 vira B2, referência interna do parser não desloca sozinha
    // A fórmula em si não foi reescrita (isso é `formula-text`/`ref_drag`,
    // outra peça) — o que este teste garante é que o grafo reconstruído
    // não deixa a `Sheet` presa num estado inconsistente: B2 continua
    // calculável, sem pânico, e A2 (o literal que só foi deslocado, sem
    // reescrever quem aponta pra ele) continua com o valor certo.
    assert_eq!(sheet.get_value(1, 0), Value::Number(7.0)); // A2
}

// `RAND`/`RANDBETWEEN`/`TODAY`/`NOW` (as funções voláteis de verdade)
// ainda não estão registradas em `functions::lookup` — o apelido em
// português existe (`functions.rs`), a implementação em si (`fn_math.rs`)
// não. Sem uma função volátil que realmente calcule, não dá para
// escrever aqui um teste que prove "recalcula toda vez" sem também
// testar essa lacuna alheia (e ficar vermelho por um motivo que não é
// deste arquivo). `is_volatile()` já tem cobertura própria em
// `sheet_deps.rs`; falta só, quando essas funções forem implementadas,
// um teste aqui que peça `get_value` duas vezes e confira que a segunda
// não veio do cache (por exemplo, uma função volátil de teste injetada).

#[test]
fn formula_invalida_nao_quebra_a_cascata_de_dependentes() {
    // Erro de compilação (não de árvore) também precisa sujar quem lê a
    // célula — `get_value` do parse-error não passa pelo avaliador, mas
    // `mark_dirty_cascade` roda do mesmo jeito.
    let mut sheet = Sheet::new("Aba1");
    sheet.set_cell_raw(0, 0, "=A1+".into(), None); // não compila
    assert!(matches!(sheet.get_value(0, 0), Value::Error(_)));

    sheet.set_cell_raw(0, 0, "=1+1".into(), None); // agora compila
    assert_eq!(sheet.get_value(0, 0), Value::Number(2.0));
}

#[test]
fn formula_simples_vai_para_formulas_table_e_nao_cells() {
    let mut sheet = Sheet::new("Aba1");
    sheet.set_cell_raw(1, 3, "=B2+C2".into(), None);
    assert_eq!(sheet.formulas.size(), 1);
    assert_eq!(sheet.cells.len(), 0);

    sheet.set_cell_raw(1, 4, "=SE(D2>200;D2*1.15;D2*1.05)".into(), None);
    assert_eq!(sheet.formulas.size(), 2);
    assert_eq!(sheet.cells.len(), 0);

    sheet.set_cell_raw(1, 5, "=SE(E2>300;\"ALTO\";\"NORMAL\")".into(), None);
    assert_eq!(sheet.formulas.size(), 3);
    assert_eq!(sheet.cells.len(), 0);
}

/// Objetos soltos (campo `ob`): cada gravação é um Ctrl+Z e não toca células.
#[test]
fn alterar_objetos_registra_um_passo_por_operacao() {
    let mut sheet = Sheet::new("Aba");
    sheet.set_cell_raw(0, 0, "10".into(), None);
    let a = Some(r#"{"v":1,"objetos":[{"id":1,"forma":"rect"}]}"#.to_string());
    let b = Some(r#"{"v":1,"objetos":[{"id":1,"forma":"ellipse"}]}"#.to_string());
    assert!(sheet.alterar_objetos(a.clone()));
    assert!(!sheet.alterar_objetos(a.clone()), "igual não cria passo");
    assert!(sheet.alterar_objetos(b.clone()));
    assert!(sheet.alterar_objetos(None));
    assert!(sheet.undo());
    assert_eq!(sheet.ob, b);
    assert!(sheet.undo());
    assert_eq!(sheet.ob, a);
    assert!(sheet.undo());
    assert_eq!(sheet.ob, None);
    assert!(sheet.redo());
    assert_eq!(sheet.ob, a);
    assert_eq!(sheet.get_raw(0, 0), "10");
    assert_eq!(sheet.duplicar("Cópia").ob, a);
}

#[test]
fn soma_de_vazio_e_zero_sem_sinal() {
    use crate::celulas::{style::format_value, values::Value};
    let mut s = Sheet::new("x");
    s.set_cell(0, 1, "=SOMA(A1:A3)");
    assert!(matches!(s.get_value(0, 1), Value::Number(n) if n == 0.0 && n.is_sign_positive()), "Rust 1.81: soma vazia de f64 é -0.0");
    assert_eq!(format_value(&Value::Number(-0.0), None), "0");
}

/// Gráficos (campo `g`): cada gravação é um Ctrl+Z; `juntar` estende a última
/// etapa (letras digitadas no mesmo campo do painel viram um passo só).
#[test]
fn alterar_graficos_um_passo_e_juntar_digitacao() {
    let mut sheet = Sheet::new("Aba");
    let a = Some(r#"[{"id":"g","title":"A"}]"#.to_string());
    let b = Some(r#"[{"id":"g","title":"AB"}]"#.to_string());
    let c = Some(r#"[{"id":"g","title":"ABC"}]"#.to_string());
    assert!(sheet.alterar_graficos(a.clone(), false));
    assert!(!sheet.alterar_graficos(a.clone(), false), "igual não cria passo");
    assert!(sheet.alterar_graficos(b.clone(), false));
    assert!(sheet.alterar_graficos(c.clone(), true), "junta com a etapa anterior");
    assert!(sheet.undo());
    assert_eq!(sheet.g, a);
    assert!(sheet.redo());
    assert_eq!(sheet.g, c);
    assert!(sheet.undo());
    assert!(sheet.undo());
    assert_eq!(sheet.g, None);
}
